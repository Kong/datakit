# DataKit

DataKit is effectively a dataflow language: a filter configuration specifies a directed graph of
operations to be performed, based on their data dependencies.

## The data model

The data types are based on those of [serde-json], so representable value types are:

* Null
* Boolean
* Number
* String
* Array (a vector of values)
* Object (a map from strings to values)

## The execution model

Nodes can have input ports and output ports.
Input ports consume data. Output ports produce data.

You can link one node's output port to another node's input port.
An input port can receive at most one link, that is, data can only arrive
into an input via one other node. Therefore, there are no race conditions.

An output port can be linked to multiple nodes. Therefore, one node can
provide data to several other nodes.

Each node triggers at most once.

A node only triggers when data is available to all its connected input ports;
that is, only when all nodes connected to its inputs have finished
executing.

## Node types

The following node types are implemented:

**Node type**        | **Input ports**   | **Output ports**  |  **Supported attributes**
--------------------:|:-----------------:|:-----------------:|:-----------------------------
`call                | `body`, `headers` | `body`, `headers` | `url`, `method`, `timeout`
`jq`                 | user-defined      | user-defined      | `jq`
`handlebars`         | user-defined      | `output`          | `template`, `content_type`
`exit`               | `body`, `headers` |                   | `status`

### `call` node type

An HTTP dispatch call.

#### Input ports:

* `body`: body to use in the dispatch request.
* `headers`: headers to use in the dispatch request.

#### Output ports:

* `body`: body returned as the dispatch response.
* `headers`: headers returned as the dispatch response.

#### Supported attributes:

* `url`: the URL to use when dispatching.
* `method`: the HTTP method (default is `GET`).
* `timeout`: the dispatch timeout, in seconds (default is 60).

### `jq` node type

Execution of a JQ script for processing JSON. The JQ script is processed
using the [jaq] implementation of the JQ language.

#### Input ports:

User-defined. Each input port declared by the user will correspond to a
variable in the JQ execution context. A user can declare the name of the port
explicitly, which is the name of the variable. If a port does not have a given
name, it will get a default name based on the peer node and port to which it
is connected, and the name will be normalized into a valid variable name (e.g.
by replacing `.` to `_`).

#### Output ports:

User-defined. When the JQ script produces a JSON value, that is made available
in the first output port of the node. If the JQ script produces multiple JSON
values, each value will be routed to a separate output port.

#### Supported attributes:

* `jq`: the JQ script to execute when the node is triggered.

### `handlebars` node type

Application of a [Handlebars] template on a raw string, useful for producing
arbitrary non-JSON content types.

#### Input ports:

User-defined. Each input port declared by the user will correspond to a
variable in the Handlebars execution context. A user can declare the name of
the port explicitly, which is the name of the variable. If a port does not
have a given name, it will get a default name based on the peer node and port
to which it is connected, and the name will be normalized into a valid
variable name (e.g. by replacing `.` to `_`).

#### Output ports:

* `output`: the rendered template. The output payload will be in raw string
  format, unless an alternative `content_type` triggers a conversion.

#### Supported attributes:

* `template`: the Handlebars template to apply when the node is triggered.
* `content_type`: if set to a MIME type that matches one of DataKit's
  supported payload types, such as `application/json`, the output payload will
  be converted to that format, making its contents available for further
  processing by other nodes (default is `text/plain`, which produces a raw
  string).

### `exit` node type

Trigger an early exit that produces a direct response, rather than forwarding
a proxied response.

#### Input ports:

* `body`: body to use in the early-exit response.
* `headers`: headers to use in the early-exit response.

#### Output ports:

None.

#### Supported attributes:

* `status`: the HTTP status code to use in the early-exit response (default is
  200).

## Implicit nodes

DataKit defines a number of implicit nodes that can be used without being
explicitly declared. These reserved node names cannot be used for user-defined
nodes. These are:

**Node**             | **Input ports**   | **Output ports**  |  **Description**
--------------------:|:-----------------:|:-----------------:|:------------------
`request`            |                   | `body`, `headers` | the incoming request
`service_request`    | `body`, `headers` |                   | request sent to the service being proxied to
`service_response`   |                   | `body`, `headers` | response sent by the service being proxied to
`response`           | `body`, `headers` |                   | response to be sent to the incoming request

The `headers` ports produce and consume maps from header names to their values.
Keys are header names are normalized to lowercase.
Values are strings if there is a single instance of a header,
or arrays of strings if there are multiple instances of the same header.

The `body` output ports produce either raw strings or JSON objects,
depending on their corresponding `Content-Type` values.

Likewise, the `body` input ports accept either raw strings or JSON objects,
and both their `Content-Type` and `Content-Length` are automatically adjusted,
according to the type and size of the incoming data.

## Debugging

DataKit includes support for debugging your configuration.

### Execution tracing

By setting the `X-DataKit-Debug-Trace` header, DataKit records the execution
flow and the values of intermediate nodes, reporting the output in the request
body in JSON format.

If the debug header value is set to `0`, `false`, or `off`, this is equivalent to
unsetting the debug header: tracing will not happen and execution will run
as normal. Any other value will enable debug tracing.

---

[serde-json]: https://docs.rs/serde_json/latest/serde_json/
[Handlebars]: https://docs.rs/handlebars/latest/handlebars/
[jaq]: https://lib.rs/crates/jaq
