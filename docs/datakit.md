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

Nodes have can have input ports and output ports.
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

* `call`: an HTTP dispatch call
* `jq`: execution of a JQ script
* `template`: application of a raw string template
* `response`: trigger a direct response, rather than forwarding a proxied response

## Implicit nodes

DataKit defines a number of implicit nodes that can be used without being
explicitly declared. These reserved node names cannot be used for user-defined nodes. These are:

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
