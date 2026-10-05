use crate::{
    config::types::GemonMethodType,
    constants::PROJECT_ROOT_FILE,
    project::{project_handler::save_request_to_project, ProjectError},
    request::rest_request::{GemonRestRequest, GemonRestRequestBuilder},
};
use serde_derive::Deserialize;
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const OPENAPI_FILE_NAME: &str = "openapi.yaml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiImportReport {
    pub specs_found: usize,
    pub requests_imported: usize,
    pub request_names: Vec<String>,
}

impl OpenApiImportReport {
    pub fn summary(&self) -> String {
        match (self.specs_found, self.requests_imported) {
            (0, _) => String::from("No openapi.yaml files found"),
            (_, 0) => format!(
                "Found {} openapi.yaml file(s), but no supported REST operations",
                self.specs_found
            ),
            _ => format!(
                "Imported {} request(s) from {} openapi.yaml file(s)",
                self.requests_imported, self.specs_found
            ),
        }
    }
}

#[derive(Debug)]
struct ImportedRequest {
    name: String,
    request: GemonRestRequest,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenApiDocument {
    #[serde(default)]
    servers: Vec<OpenApiServer>,
    #[serde(default)]
    schemes: Vec<String>,
    host: Option<String>,
    base_path: Option<String>,
    #[serde(default)]
    paths: BTreeMap<String, PathItem>,
}

#[derive(Debug, Deserialize)]
struct OpenApiServer {
    url: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathItem {
    #[serde(default)]
    parameters: Vec<Parameter>,
    get: Option<Operation>,
    post: Option<Operation>,
    put: Option<Operation>,
    patch: Option<Operation>,
    delete: Option<Operation>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Operation {
    operation_id: Option<String>,
    #[serde(default)]
    parameters: Vec<Parameter>,
    request_body: Option<RequestBody>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Parameter {
    name: Option<String>,
    #[serde(rename = "in")]
    location: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RequestBody {
    #[serde(default)]
    content: BTreeMap<String, MediaType>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaType {
    example: Option<YamlValue>,
    #[serde(default)]
    examples: BTreeMap<String, Example>,
    schema: Option<Schema>,
}

#[derive(Debug, Default, Deserialize)]
struct Example {
    value: Option<YamlValue>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Schema {
    #[serde(rename = "type")]
    schema_type: Option<String>,
    #[serde(default)]
    properties: BTreeMap<String, Schema>,
    items: Option<Box<Schema>>,
    example: Option<YamlValue>,
    default: Option<YamlValue>,
    #[serde(rename = "enum", default)]
    enumeration: Vec<YamlValue>,
    #[serde(default)]
    all_of: Vec<Schema>,
    #[serde(default)]
    one_of: Vec<Schema>,
    #[serde(default)]
    any_of: Vec<Schema>,
    #[serde(rename = "$ref")]
    reference: Option<String>,
}

pub fn import_openapi_requests() -> Result<OpenApiImportReport, Box<dyn Error>> {
    import_openapi_requests_from(Path::new("."))
}

pub(crate) fn import_openapi_requests_from(
    root: &Path,
) -> Result<OpenApiImportReport, Box<dyn Error>> {
    if !root.join(PROJECT_ROOT_FILE).exists() {
        return Err(ProjectError::from("Project not found!"));
    }

    let spec_paths = find_openapi_specs(root)?;
    let mut imported_names = Vec::new();
    let mut used_names = HashSet::new();

    for spec_path in &spec_paths {
        for imported in requests_from_spec(spec_path, &mut used_names)? {
            let name = name_outside_foreign_paths(root, imported.name, &mut used_names);
            save_request_to_project(root, &imported.request, &name)?;
            imported_names.push(name);
        }
    }

    Ok(OpenApiImportReport {
        specs_found: spec_paths.len(),
        requests_imported: imported_names.len(),
        request_names: imported_names,
    })
}

/// Whether `name` is taken by something that is not a saved request, e.g. the folder
/// holding the spec. Requests are never written into such paths.
fn is_foreign_path(root: &Path, name: &str) -> bool {
    let path = root.join(name);
    path.exists() && !path.join(".marker").exists()
}

fn name_outside_foreign_paths(root: &Path, name: String, used_names: &mut HashSet<String>) -> String {
    if !is_foreign_path(root, &name) {
        return name;
    }
    (2..)
        .map(|index| format!("{name}_{index}"))
        .find(|candidate| !is_foreign_path(root, candidate) && used_names.insert(candidate.clone()))
        .expect("an unused name exists")
}

fn find_openapi_specs(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut specs = Vec::new();
    collect_openapi_specs(root, &mut specs)?;
    specs.sort();
    Ok(specs)
}

fn collect_openapi_specs(dir: &Path, specs: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_openapi_specs(&path, specs)?;
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some(OPENAPI_FILE_NAME)
        {
            specs.push(path);
        }
    }

    Ok(())
}

fn requests_from_spec(
    spec_path: &Path,
    used_names: &mut HashSet<String>,
) -> Result<Vec<ImportedRequest>, Box<dyn Error>> {
    let spec = fs::read_to_string(spec_path)?;
    let document: OpenApiDocument = serde_yaml::from_str(&spec)?;
    let base_url = document_base_url(&document);
    let mut requests = Vec::new();

    for (path, item) in &document.paths {
        for (method, operation) in operation_entries(item) {
            let name = request_name(method, path, operation, used_names);
            let parameters = item
                .parameters
                .iter()
                .chain(operation.parameters.iter())
                .cloned()
                .collect::<Vec<_>>();
            let url = request_url(&base_url, path, &parameters);
            let headers = header_parameters(&parameters);
            let (body, form_data) = request_payload(operation);
            let request = GemonRestRequestBuilder::new()
                .set_gemon_method_type(method)
                .set_url(url)
                .set_headers(&headers)
                .set_body(body)
                .set_form_data(&form_data)
                .build();
            requests.push(ImportedRequest { name, request });
        }
    }

    Ok(requests)
}

fn operation_entries(item: &PathItem) -> Vec<(GemonMethodType, &Operation)> {
    [
        (GemonMethodType::Get, item.get.as_ref()),
        (GemonMethodType::Post, item.post.as_ref()),
        (GemonMethodType::Put, item.put.as_ref()),
        (GemonMethodType::Patch, item.patch.as_ref()),
        (GemonMethodType::Delete, item.delete.as_ref()),
    ]
    .into_iter()
    .filter_map(|(method, operation)| operation.map(|operation| (method, operation)))
    .collect()
}

fn document_base_url(document: &OpenApiDocument) -> String {
    if let Some(server) = document
        .servers
        .iter()
        .find(|server| !server.url.trim().is_empty())
    {
        return server.url.trim().to_string();
    }

    if let Some(host) = document
        .host
        .as_deref()
        .filter(|host| !host.trim().is_empty())
    {
        let scheme = document
            .schemes
            .first()
            .map(String::as_str)
            .unwrap_or("https");
        let base_path = document.base_path.as_deref().unwrap_or_default();
        return join_url_path(&format!("{scheme}://{}", host.trim()), base_path);
    }

    String::from("{base_uri}")
}

fn request_name(
    method: GemonMethodType,
    path: &str,
    operation: &Operation,
    used_names: &mut HashSet<String>,
) -> String {
    let base_name = operation
        .operation_id
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_{}", method.as_str().to_lowercase(), path));
    let sanitized = sanitize_request_name(&base_name);

    if used_names.insert(sanitized.clone()) {
        return sanitized;
    }

    for index in 2.. {
        let candidate = format!("{sanitized}_{index}");
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
    }

    unreachable!("deduplicating request names always returns");
}

fn sanitize_request_name(name: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_separator = false;

    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character);
            last_was_separator = false;
        } else if !last_was_separator {
            sanitized.push('_');
            last_was_separator = true;
        }
    }

    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        String::from("openapi_request")
    } else {
        sanitized
    }
}

fn request_url(base_url: &str, path: &str, parameters: &[Parameter]) -> String {
    let mut url = join_url_path(base_url, path);
    let query = parameters
        .iter()
        .filter(|parameter| parameter.location.as_deref() == Some("query"))
        .filter_map(|parameter| parameter.name.as_deref())
        .filter(|name| !name.trim().is_empty())
        .map(|name| {
            let name = name.trim();
            format!("{name}={{{name}}}")
        })
        .collect::<Vec<_>>();

    if !query.is_empty() {
        let separator = if url.contains('?') { '&' } else { '?' };
        url.push(separator);
        url.push_str(&query.join("&"));
    }

    url
}

fn join_url_path(base_url: &str, path: &str) -> String {
    let base_url = base_url.trim();
    let path = path.trim();

    if path.is_empty() {
        return base_url.to_string();
    }

    if base_url.is_empty() {
        return path.to_string();
    }

    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn header_parameters(parameters: &[Parameter]) -> HashMap<String, String> {
    parameters
        .iter()
        .filter(|parameter| parameter.location.as_deref() == Some("header"))
        .filter_map(|parameter| parameter.name.as_deref())
        .filter(|name| !name.trim().is_empty())
        .map(|name| {
            let name = name.trim().to_string();
            (name.clone(), format!("{{{name}}}"))
        })
        .collect()
}

fn request_payload(operation: &Operation) -> (Option<String>, HashMap<String, String>) {
    let Some(request_body) = operation.request_body.as_ref() else {
        return (None, HashMap::new());
    };

    if let Some(media_type) = request_body
        .content
        .get("application/x-www-form-urlencoded")
        .or_else(|| request_body.content.get("multipart/form-data"))
    {
        return (None, form_data_from_media_type(media_type));
    }

    let Some(media_type) = request_body
        .content
        .get("application/json")
        .or_else(|| request_body.content.values().next())
    else {
        return (None, HashMap::new());
    };

    let body = json_value_from_media_type(media_type)
        .and_then(|value| serde_json::to_string_pretty(&value).ok());
    (body, HashMap::new())
}

fn form_data_from_media_type(media_type: &MediaType) -> HashMap<String, String> {
    media_type
        .schema
        .as_ref()
        .map(|schema| {
            schema
                .properties
                .keys()
                .map(|key| (key.clone(), format!("{{{key}}}")))
                .collect()
        })
        .unwrap_or_default()
}

fn json_value_from_media_type(media_type: &MediaType) -> Option<JsonValue> {
    media_type
        .example
        .as_ref()
        .and_then(yaml_to_json)
        .or_else(|| {
            media_type
                .examples
                .values()
                .find_map(|example| example.value.as_ref().and_then(yaml_to_json))
        })
        .or_else(|| {
            media_type
                .schema
                .as_ref()
                .map(|schema| json_from_schema(None, schema))
        })
}

fn json_from_schema(name: Option<&str>, schema: &Schema) -> JsonValue {
    if let Some(value) = schema.example.as_ref().and_then(yaml_to_json) {
        return value;
    }

    if let Some(value) = schema.default.as_ref().and_then(yaml_to_json) {
        return value;
    }

    if let Some(value) = schema.enumeration.first().and_then(yaml_to_json) {
        return value;
    }

    if let Some(schema) = schema
        .all_of
        .iter()
        .chain(schema.one_of.iter())
        .chain(schema.any_of.iter())
        .next()
    {
        return json_from_schema(name, schema);
    }

    if schema.reference.is_some() {
        return placeholder_value(name);
    }

    match schema.schema_type.as_deref() {
        Some("object") | None if !schema.properties.is_empty() => {
            let properties = schema
                .properties
                .iter()
                .map(|(key, value)| (key.clone(), json_from_schema(Some(key), value)))
                .collect::<JsonMap<_, _>>();
            JsonValue::Object(properties)
        }
        Some("array") => JsonValue::Array(vec![schema
            .items
            .as_deref()
            .map(|items| json_from_schema(name, items))
            .unwrap_or(JsonValue::Null)]),
        Some("integer") => JsonValue::Number(JsonNumber::from(0)),
        Some("number") => JsonNumber::from_f64(0.0)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Some("boolean") => JsonValue::Bool(false),
        Some("string") => placeholder_value(name),
        _ => placeholder_value(name),
    }
}

fn placeholder_value(name: Option<&str>) -> JsonValue {
    JsonValue::String(
        name.filter(|name| !name.trim().is_empty())
            .map(|name| format!("{{{}}}", name.trim()))
            .unwrap_or_default(),
    )
}

fn yaml_to_json(value: &YamlValue) -> Option<JsonValue> {
    serde_json::to_value(value).ok()
}

#[cfg(test)]
mod tests {
    use super::{find_openapi_specs, import_openapi_requests_from};
    use crate::{
        config::types::GemonMethodType, constants::PROJECT_ROOT_FILE,
        project::project_handler::read_saved_rest_request_from,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_dir(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "gemon_openapi_import_{name}_{}_{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&path).expect("create temp test dir");
        path
    }

    fn write_project(root: &Path) {
        fs::write(
            root.join(PROJECT_ROOT_FILE),
            r#"{
  "name": "test",
  "selected_environment": null,
  "environments": {},
  "authorization": {},
  "last_called_request_path": null
}"#,
        )
        .expect("write project file");
    }

    #[test]
    fn finds_openapi_yaml_recursively() {
        let root = temp_dir("finds");
        fs::create_dir_all(root.join("docs/api")).expect("create nested dir");
        fs::write(root.join("openapi.yaml"), "openapi: 3.0.0\npaths: {}\n")
            .expect("write root spec");
        fs::write(
            root.join("docs/api/openapi.yaml"),
            "openapi: 3.0.0\npaths: {}\n",
        )
        .expect("write nested spec");

        let specs = find_openapi_specs(&root).expect("find specs");

        assert_eq!(specs.len(), 2);
        assert!(specs.iter().any(|path| path == &root.join("openapi.yaml")));
        assert!(specs
            .iter()
            .any(|path| path == &root.join("docs/api/openapi.yaml")));

        fs::remove_dir_all(root).expect("clean temp test dir");
    }

    #[test]
    fn imports_openapi_operations_as_saved_rest_requests() {
        let root = temp_dir("imports");
        write_project(&root);
        fs::create_dir_all(root.join("docs")).expect("create docs dir");
        fs::write(
            root.join("docs/openapi.yaml"),
            r#"
openapi: 3.0.0
servers:
  - url: https://api.example.com/v1
paths:
  /pets/{petId}:
    parameters:
      - name: X-Correlation-ID
        in: header
    get:
      operationId: getPet
      parameters:
        - name: verbose
          in: query
    post:
      operationId: createPet
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                name:
                  type: string
                age:
                  type: integer
"#,
        )
        .expect("write spec");

        let report = import_openapi_requests_from(&root).expect("import requests");

        assert_eq!(report.specs_found, 1);
        assert_eq!(report.requests_imported, 2);
        assert_eq!(
            report.request_names,
            vec![String::from("getPet"), String::from("createPet")]
        );

        let get_pet =
            read_saved_rest_request_from(&root, "getPet").expect("read imported get request");
        assert_eq!(get_pet.method(), GemonMethodType::Get);
        assert_eq!(
            get_pet.uri(),
            "https://api.example.com/v1/pets/{petId}?verbose={verbose}"
        );
        assert_eq!(
            get_pet.headers().get("X-Correlation-ID"),
            Some(&String::from("{X-Correlation-ID}"))
        );

        let create_pet =
            read_saved_rest_request_from(&root, "createPet").expect("read imported post request");
        assert_eq!(create_pet.method(), GemonMethodType::Post);
        assert_eq!(
            create_pet.body(),
            Some("{\n  \"age\": 0,\n  \"name\": \"{name}\"\n}")
        );

        fs::remove_dir_all(root).expect("clean temp test dir");
    }

    #[test]
    fn import_never_writes_into_folders_that_are_not_requests() {
        let root = temp_dir("foreign");
        write_project(&root);
        fs::create_dir_all(root.join("api")).expect("create spec dir");
        fs::write(
            root.join("api/openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /api:\n    get:\n      operationId: api\n",
        )
        .expect("write spec");

        let report = import_openapi_requests_from(&root).expect("import requests");

        assert_eq!(report.request_names, vec![String::from("api_2")]);
        assert!(!root.join("api/.marker").exists());
        assert!(root.join("api_2/.marker").exists());

        fs::remove_dir_all(root).expect("clean temp test dir");
    }
}
