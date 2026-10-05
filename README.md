# Gemon

Gemon is a Rust-based terminal tool designed to facilitate API testing, functioning as a command-line alternative to Postman. It supports REST endpoint calls and plans to include WebSocket and Protobuf testing in the future. Gemon allows users to execute API calls directly through the terminal or create project files for efficient and organized testing.

## Features

* Make REST API calls directly from the terminal.
* Save and manage environment variables for dynamic request customization.
* Store and organize requests in project files for easy reuse and testing.
* Print and save API responses for later review and debugging.

## Installation

To install Gemon, clone the repository and build the project using Cargo:

```sh
git clone https://github.com/ehasanaj/gemon.git
cd gemon
cargo build --release
```

Add the binary to your PATH for easy access:

```sh
export PATH=$PATH:/path/to/gemon/target/release
```

## Usage

Gemon supports various commands and options for making API requests, managing environments, and organizing requests. Below is a detailed guide on how to use Gemon.

### Basic Commands

```sh
-h | --help : Print the list of command options in the terminal.
-v | --version : Print Gemon version information.
tui | --tui | -i | --interactive : Open the interactive terminal user interface.
import-openapi | --import-openapi : Import REST requests from openapi.yaml files in this project.
```

### Interactive TUI

Open Gemon's terminal user interface in a project folder:

```sh
gemon tui
```

The TUI covers everything the CLI does — creating a project, composing, sending, saving and
organizing requests, environments and their variables, authorization, OpenAPI import, and saving
responses — and it is fully usable with the keyboard alone. Requests are sent in the background:
the interface stays responsive and `Esc` cancels a slow request.

The screen is split into the saved-request list, the URL bar (method + URL), a tabbed request
editor (Headers, Body, Form, Auth) and the response viewer (Body, Headers). `Tab` and
`Shift-Tab` move between panels, and the bottom bar always shows the keys for the focused panel.
Press `Ctrl-P` for a searchable list of every command, or `F3` (`?` outside text fields) for
the full shortcut reference.

Everywhere:

```sh
Tab / Shift-Tab   next / previous panel     Ctrl-P   command palette
Ctrl-R            send request              Ctrl-S   save request
Ctrl-N            new request               Ctrl-L   edit URL
Ctrl-F            search saved requests     Ctrl-T   choose method
Ctrl-G            switch environment        Ctrl-O   import openapi.yaml
F1 / F2           requests / environments   F3, ?    help
Esc               close, clear, cancel      Ctrl-Q   quit (asks about unsaved changes)
```

In panels:

```sh
URL bar           Enter sends the request
Request list      Enter open, / filter, n new, r rename, c duplicate, d delete, < > resize
Headers / Form    a add, Enter edit, d delete
Body              Enter keeps indentation, braces indent automatically
Auth              Space toggles sending the environment's authorization, e edits it
Response          arrows scroll, ←→ body/headers, / search (n/N), y copy, s save, z full screen
Environments      Enter activate, n new, r rename, d delete, a add variable, u authorization
Text fields       Ctrl-A/E line start/end, Ctrl-U/K delete to start/end, Ctrl-W delete word
```

Environment placeholders such as `{base_uri}` are resolved when a request is sent; the URL bar
shows the resolved URL and warns about placeholders the active environment does not define. The
TUI picks up changes made with the CLI in another terminal automatically. Mouse clicks, the
wheel, and dragging panel borders work too, but are never required.

### OpenAPI Import

Import REST requests from OpenAPI documentation saved as `openapi.yaml` in the project root or
any child directory:

```sh
gemon import-openapi
```

The importer scans the project tree recursively, converts supported OpenAPI path operations into
saved REST requests, and uses the first server URL from the specification. If no server URL is
defined, imported URLs use `{base_uri}` so existing Gemon environments can provide the host.

### Project Initialization

Initialize the current folder as a Gemon project:

```sh
gemon init
```

### Environment Management

Print all environments with their associated variables:

```sh
gemon print-env-all
```

Print values of the current environment:

```sh
gemon print-env
```

Save a new environment variable:

```sh
gemon -e=(env_name::variable_name::value)
```

Delete an environment:

```sh
gemon -ed=(env_name)
```

Remove an environment variable:

```sh
gemon -edv=(env_name::variable_name) | --env-delete-value=(env_name::variable_name)
```

Select a previously created environment as the current environment:

```sh
gemon -se=(env_name)
```

Remove authorization for selcted env (if no env selcted remove default authorization)

```sh
gemon -r-auth | --remove-authorization
```

### Making API Calls

Set the request type:

```sh
gemon -t=(REST | WEBSOCKET | PROTO)
```

Set the REST method (required when -t=REST):

```sh
gemon -m=(GET | POST | DELETE | PUT | PATCH)
```

Set the URI of the request:

```sh
gemon -u=(https://api.com:8080) | --uri=(https://api.com:8080)
```

Add a header to the request:

```sh
gemon -h=(key::value) | --header=(key::value)
```

Set the body of the request:

```sh
gemon -b=('{"name": "some name"}') | --body=('{"name": "some name"}')
```

Set a form data parameter:

```sh
gemon -fd=(key::value) | --form-data=(key::value)
```

Set authorization for selected env (if no env selected set default authorization)

```sh
gemon -auth='Bearer token...' | --authorization='Bearer token...'
```

Mark request secured that needs to be authorized. The authorization of the selected environment
(or the default one) is added when the request is sent, unless an `authorization` header is set.
Saved requests remember this flag.

```sh
gemon -sec | --secure
```

### Response Handling

Save the response to the default response.json file:

```sh
gemon -f | --file
```

Save the response with a timestamp:

```sh
gemon -l | --log
```

Save the response to a file and print it to the terminal:

```sh
gemon -p | --print
```

Save the response to a specified file:

```sh
gemon -rf=(file_name.json) | --response-file=(file_name.json)
```

### Request Management

Save the request into the project for future calls:

```sh
gemon -s=(request_name) | --save=(request_name)
```

Call a previously saved request:

```sh
gemon -c=(request_name) | --call=(request_name)
```

Simultaneously save a new request and call it:

```sh
gemon -sc=(request_name) | --save-and-call=(request_name)
```

Remove a previously saved request:

```sh
gemon -d=(request_name) | --delete=(request_name)
```

### Printing Responses

Print the last call response stored in the file:

```sh
gemon print
```

## Example

Here's an example of how to use Gemon to make a GET request to an API and save the response:

```sh
gemon init
gemon -t=REST -m=GET -u=https://api.example.com/data -h=Authorization::Bearer your_token -f -p
```

## Contributing

Gemon is an open-source project, and contributions are welcome! To contribute, please follow refere to CONTRIBUTING.md

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Contact

For questions or suggestions, feel free to open an issue on GitHub or contact the project maintainers at `tech.gemon@gmail.com`.

---
By following this README, you should be able to effectively utilize Gemon for your API testing needs. For more detailed information, refer to the help command or the source code documentation.
