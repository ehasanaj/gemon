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
```

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
gemon -edv=(env_name::variable_name)
```

Select a previously created environment as the current environment:

```sh
gemon -se=(env_name)
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
gemon -fd=(key:value) | --form-data=(key:value)
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

Save the response into the project for future calls:

```sh
gemon -s | --save
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
