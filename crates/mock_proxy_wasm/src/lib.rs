use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::ImplItem::*;

#[proc_macro_attribute]
pub fn mock_proxy_wasm_context(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_mock_proxy_wasm_context(&ast)
}

fn impl_mock_proxy_wasm_context(ast: &syn::ItemImpl) -> proc_macro::TokenStream {
    let self_ty = &ast.self_ty;
    let mut used: HashMap<String, proc_macro2::TokenStream> = HashMap::new();
    for item in &ast.items {
        if let Fn(f) = item {
            used.insert(f.sig.ident.to_string(), item.into_token_stream());
        }
    }
    let mut gen = proc_macro2::TokenStream::new();

    let mock = quote! {
            todo!("mock function")
    };

    match used.get("get_property") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_property(&self, path: Vec<&str>) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_property") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_property(&self, path: Vec<&str>, value: Option<&[u8]>) {
                    #mock
                }
            });
        }
    }

    match used.get("get_current_time") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_current_time(&self) -> std::time::SystemTime {
                    #mock
                }
            });
        }
    }

    match used.get("get_shared_data") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_shared_data(&self, _key: &str) -> (Option<Bytes>, Option<u32>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_shared_data") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_shared_data(
                    &self,
                    _key: &str,
                    _value: Option<&[u8]>,
                    _cas: Option<u32>,
                ) -> Result<(), proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("register_shared_queue") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn register_shared_queue(&self, _name: &str) -> u32 {
                    #mock
                }
            });
        }
    }

    match used.get("resolve_shared_queue") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn resolve_shared_queue(&self, _vm_id: &str, _name: &str) -> Option<u32> {
                    #mock
                }
            });
        }
    }

    match used.get("dequeue_shared_queue") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn dequeue_shared_queue(
                    &self,
                    _queue_id: u32,
                ) -> Result<Option<Bytes>, proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("enqueue_shared_queue") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn enqueue_shared_queue(
                    &self,
                    _queue_id: u32,
                    _value: Option<&[u8]>,
                ) -> Result<(), proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("dispatch_http_call") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn dispatch_http_call(
                    &self,
                    _upstream: &str,
                    _headers: Vec<(&str, &str)>,
                    _body: Option<&[u8]>,
                    _trailers: Vec<(&str, &str)>,
                    _timeout: std::time::Duration,
                ) -> Result<u32, proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("on_http_call_response") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_http_call_response(
                    &mut self,
                    _token_id: u32,
                    _num_headers: usize,
                    _body_size: usize,
                    _num_trailers: usize,
                ) {
                }
            });
        }
    }

    match used.get("get_http_call_response_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_headers(&self) -> Vec<(String, String)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_headers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_headers_bytes(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_header(&self, _name: &str) -> Option<String> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_header_bytes(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_trailers(&self) -> Vec<(String, String)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_trailers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_trailers_bytes(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_trailer(&self, _name: &str) -> Option<String> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_call_response_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_call_response_trailer_bytes(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("dispatch_grpc_call") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn dispatch_grpc_call(
                    &self,
                    _upstream_name: &str,
                    _service_name: &str,
                    _method_name: &str,
                    _initial_metadata: Vec<(&str, &[u8])>,
                    _message: Option<&[u8]>,
                    _timeout: std::time::Duration,
                ) -> Result<u32, proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("on_grpc_call_response") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_grpc_call_response(&mut self, _token_id: u32, _status_code: u32, _response_size: usize) {}
            });
        }
    }

    match used.get("get_grpc_call_response_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_grpc_call_response_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("cancel_grpc_call") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn cancel_grpc_call(&self, _token_id: u32) {
                    #mock
                }
            });
        }
    }

    match used.get("open_grpc_stream") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn open_grpc_stream(
                    &self,
                    _cluster_name: &str,
                    _service_name: &str,
                    _method_name: &str,
                    _initial_metadata: Vec<(&str, &[u8])>,
                ) -> Result<u32, proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("on_grpc_stream_initial_metadata") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_grpc_stream_initial_metadata(&mut self, _token_id: u32, _num_elements: u32) {}
            });
        }
    }

    match used.get("get_grpc_stream_initial_metadata") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_grpc_stream_initial_metadata(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_grpc_stream_initial_metadata_value") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_grpc_stream_initial_metadata_value(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("send_grpc_stream_message") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn send_grpc_stream_message(&self, _token_id: u32, _message: Option<&[u8]>, _end_stream: bool) {
                    #mock
                }
            });
        }
    }

    match used.get("on_grpc_stream_message") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_grpc_stream_message(&mut self, _token_id: u32, _message_size: usize) {}
            });
        }
    }

    match used.get("get_grpc_stream_message") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
    fn get_grpc_stream_message(&mut self, _start: usize, _max_size: usize) -> Option<Bytes> {
        #mock
    }
});
        }
    }

    match used.get("on_grpc_stream_trailing_metadata") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
            fn on_grpc_stream_trailing_metadata(&mut self, _token_id: u32, _num_elements: u32) {}
        });
        }
    }

    match used.get("get_grpc_stream_trailing_metadata") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_grpc_stream_trailing_metadata(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_grpc_stream_trailing_metadata_value") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_grpc_stream_trailing_metadata_value(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("cancel_grpc_stream") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn cancel_grpc_stream(&self, _token_id: u32) {
                    #mock
                }
            });
        }
    }

    match used.get("close_grpc_stream") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn close_grpc_stream(&self, _token_id: u32) {
                    #mock
                }
            });
        }
    }

    match used.get("on_grpc_stream_close") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_grpc_stream_close(&mut self, _token_id: u32, _status_code: u32) {}
            });
        }
    }

    match used.get("get_grpc_status") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_grpc_status(&self) -> (u32, Option<String>) {
                    #mock
                }
            });
        }
    }

    match used.get("call_foreign_function") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn call_foreign_function(
                    &self,
                    _function_name: &str,
                    _arguments: Option<&[u8]>,
                ) -> Result<Option<Bytes>, proxy_wasm::types::Status> {
                    #mock
                }
            });
        }
    }

    match used.get("on_done") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_done(&mut self) -> bool {
                    true
                }
            });
        }
    }

    match used.get("done") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn done(&self) {
                    #mock
                }
            });
        }
    }

    let out = quote! {
        impl Context for #self_ty {
            #gen
        }
    };

    out.into()
}

#[proc_macro_attribute]
pub fn mock_proxy_wasm_http_context(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_mock_proxy_wasm_http_context(&ast)
}

fn impl_mock_proxy_wasm_http_context(ast: &syn::ItemImpl) -> proc_macro::TokenStream {
    let self_ty = &ast.self_ty;
    let mut used: HashMap<String, proc_macro2::TokenStream> = HashMap::new();
    for item in &ast.items {
        if let Fn(f) = item {
            used.insert(f.sig.ident.to_string(), item.into_token_stream());
        }
    }
    let mut gen = proc_macro2::TokenStream::new();

    let mock = quote! {
            todo!("mock function")
    };

    match used.get("on_http_request_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_http_request_headers(
                    &mut self,
                    _num_headers: usize,
                    _end_of_stream: bool,
                ) -> proxy_wasm::types::Action {
                    proxy_wasm::types::Action::Continue
                }
            });
        }
    }

    match used.get("get_http_request_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_headers(&self) -> Vec<(String, String)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_request_headers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_headers_bytes(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_headers(&self, _headers: Vec<(&str, &str)>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_headers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_headers_bytes(&self, _headers: Vec<(&str, &[u8])>) {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_request_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_header(&self, _name: &str) -> Option<String> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_request_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_header_bytes(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_header(&self, _name: &str, _value: Option<&str>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_header_bytes(&self, _name: &str, _value: Option<&[u8]>) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_request_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_request_header(&self, _name: &str, _value: &str) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_request_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_request_header_bytes(&self, _name: &str, _value: &[u8]) {
                    #mock
                }
            });
        }
    }

    match used.get("on_http_request_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_http_request_body(
                    &mut self,
                    _body_size: usize,
                    _end_of_stream: bool,
                ) -> proxy_wasm::types::Action {
                    proxy_wasm::types::Action::Continue
                }
            });
        }
    }

    match used.get("get_http_request_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_body(&self, _start: usize, _size: usize, _value: &[u8]) {
                    #mock
                }
            });
        }
    }

    match used.get("on_http_request_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
        fn on_http_request_trailers(&mut self, _num_trailers: usize) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }
            });
        }
    }

    match used.get("get_http_request_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_trailers(&self) -> Vec<(String, String)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_request_trailers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_trailers_bytes(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_trailers(&self, _trailers: Vec<(&str, &str)>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_trailers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_trailers_bytes(&self, _trailers: Vec<(&str, &[u8])>) {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_request_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_trailer(&self, _name: &str) -> Option<String> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_request_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_request_trailer_bytes(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_trailer(&self, _name: &str, _value: Option<&str>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_request_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_request_trailer_bytes(&self, _name: &str, _value: Option<&[u8]>) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_request_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_request_trailer(&self, _name: &str, _value: &str) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_request_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_request_trailer_bytes(&self, _name: &str, _value: &[u8]) {
                    #mock
                }
            });
        }
    }

    match used.get("resume_http_request") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn resume_http_request(&self) {
                    #mock
                }
            });
        }
    }

    match used.get("reset_http_request") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn reset_http_request(&self) {
                    #mock
                }
            });
        }
    }

    match used.get("on_http_response_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_http_response_headers(
                    &mut self,
                    _num_headers: usize,
                    _end_of_stream: bool,
                ) -> proxy_wasm::types::Action {
                    proxy_wasm::types::Action::Continue
                }
            });
        }
    }

    match used.get("get_http_response_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_headers(&self) -> Vec<(String, String)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_response_headers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_headers_bytes(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_headers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_headers(&self, _headers: Vec<(&str, &str)>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_headers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_headers_bytes(&self, _headers: Vec<(&str, &[u8])>) {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_response_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_header(&self, _name: &str) -> Option<String> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_response_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_header_bytes(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_header(&self, _name: &str, _value: Option<&str>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_header_bytes(&self, _name: &str, _value: Option<&[u8]>) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_response_header") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_response_header(&self, _name: &str, _value: &str) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_response_header_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_response_header_bytes(&self, _name: &str, _value: &[u8]) {
                    #mock
                }
            });
        }
    }

    match used.get("on_http_response_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_http_response_body(
                    &mut self,
                    _body_size: usize,
                    _end_of_stream: bool,
                ) -> proxy_wasm::types::Action {
                    proxy_wasm::types::Action::Continue
                }
            });
        }
    }

    match used.get("get_http_response_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_body(&self, _start: usize, _max_size: usize) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_body") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_body(&self, _start: usize, _size: usize, _value: &[u8]) {
                    #mock
                }
            });
        }
    }

    match used.get("on_http_response_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
        fn on_http_response_trailers(&mut self, _num_trailers: usize) -> proxy_wasm::types::Action {
            proxy_wasm::types::Action::Continue
        }
            });
        }
    }

    match used.get("get_http_response_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_trailers(&self) -> Vec<(String, String)> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_response_trailers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_trailers_bytes(&self) -> Vec<(String, Bytes)> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_trailers") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_trailers(&self, _trailers: Vec<(&str, &str)>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_trailers_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_trailers_bytes(&self, _trailers: Vec<(&str, &[u8])>) {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_response_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_trailer(&self, _name: &str) -> Option<String> {
                    #mock
                }
            });
        }
    }

    match used.get("get_http_response_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn get_http_response_trailer_bytes(&self, _name: &str) -> Option<Bytes> {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_trailer(&self, _name: &str, _value: Option<&str>) {
                    #mock
                }
            });
        }
    }

    match used.get("set_http_response_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn set_http_response_trailer_bytes(&self, _name: &str, _value: Option<&[u8]>) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_response_trailer") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_response_trailer(&self, _name: &str, _value: &str) {
                    #mock
                }
            });
        }
    }

    match used.get("add_http_response_trailer_bytes") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn add_http_response_trailer_bytes(&self, _name: &str, _value: &[u8]) {
                    #mock
                }
            });
        }
    }

    match used.get("resume_http_response") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn resume_http_response(&self) {
                    #mock
                }
            });
        }
    }

    match used.get("reset_http_response") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn reset_http_response(&self) {
                    #mock
                }
            });
        }
    }

    match used.get("send_http_response") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn send_http_response(
                    &self,
                    _status_code: u32,
                    _headers: Vec<(&str, &str)>,
                    _body: Option<&[u8]>,
                ) {
                    #mock
                }
            });
        }
    }

    match used.get("send_grpc_response") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn send_grpc_response(
                    &self,
                    _grpc_status: proxy_wasm::types::GrpcStatusCode,
                    _grpc_status_message: Option<&str>,
                    _custom_metadata: Vec<(&str, &[u8])>,
                ) {
                    #mock
                }
            });
        }
    }

    match used.get("on_log") {
        Some(f) => gen.extend(f.to_token_stream()),
        None => {
            gen.extend(quote! {
                fn on_log(&mut self) {}
            });
        }
    }

    let out = quote! {
        impl HttpContext for #self_ty {
            #gen
        }
    };

    out.into()
}
