/*
	Copyright 2009-2020, Sumeet Chhetri

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

        http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/
#![deny(warnings)]
extern crate libc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, StatusCode, Version, Response, Server};
use libc::{c_char, c_int, size_t, c_void};
use std::ptr;
use std::mem;
use std::slice;
use std::env;
use hyper::header::{HeaderValue, SERVER};
use std::path::Path;
use std::fs;
use futures::executor;
use std::cell::RefCell;

static NOTFOUND: &[u8] = b"Not Found";
static SERVER_NAME: &str = "hyper";

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct phr_header {
    pub name: *mut c_char,
    pub name_len: size_t,
    pub value: *mut c_char,
    pub value_len: size_t,
}
impl Default for phr_header {
    fn default() -> phr_header {
        phr_header {
            name: ptr::null_mut(),
            name_len: 0,
            value: ptr::null_mut(),
            value_len: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ffead_request {
    pub server_str: *mut c_char,
    pub server_str_len: size_t,
    pub method: *mut c_char,
    pub method_len: size_t,
    pub path: *mut c_char,
    pub path_len: size_t,
    pub headers: *mut phr_header,
    pub headers_len: c_int,
    pub body: *mut c_char,
    pub body_len: size_t,
    pub version: c_int
}
impl Default for ffead_request {
    fn default() -> ffead_request {
        ffead_request {
            server_str: ptr::null_mut(),
            server_str_len: 0,
            method: ptr::null_mut(),
            method_len: 0,
            path: ptr::null_mut(),
            path_len: 0,
            headers: ptr::null_mut(),
            headers_len: 0,
            body: ptr::null_mut(),
            body_len: 0,
            version: 0
        }
    }
}

pub struct FfeadResponse {
    pub status_code: c_int,
    pub headers: [phr_header; 100],
    pub headers_len: c_int,
    pub body: *const c_char,
    pub body_len: size_t,
    pub url: *const c_char,
    pub url_len: size_t,
}
impl Default for FfeadResponse {
    fn default() -> FfeadResponse {
        FfeadResponse {
            status_code: 0,
            headers: [phr_header::default(); 100],
            headers_len: 0,
            body: ptr::null(),
            body_len: 0,
            url: ptr::null(),
            url_len: 0
        }
    }
}

#[link(name = "ffead-framework")]
extern "C" {
    pub fn ffead_cpp_bootstrap(srv: *const c_char, srv_len: size_t, server_type: c_int);
    pub fn ffead_cpp_init();
	pub fn ffead_cpp_is_inited() -> bool;
    pub fn ffead_cpp_cleanup();
    //TOFO may need mime types
    pub fn ffead_cpp_handle_rust_1(ffead_request: *const ffead_request, status_code: *mut c_int,
        out_url: *mut *const c_char, out_url_len: *mut size_t, out_headers: *mut phr_header, out_headers_len: *mut c_int, 
        out_body: *mut *const c_char, out_body_len: *mut size_t
    ) -> *mut c_void;
    pub fn ffead_cpp_resp_cleanup(ptr: *mut c_void);
}

fn slice_from_raw<'a>(pointer: *const c_char, len: size_t) -> &'a [u8] {
    unsafe { mem::transmute(slice::from_raw_parts(pointer, len)) }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

async fn to_bytes(_req: Request<Body>) -> Result<bytes::Bytes, hyper::Error> {
    hyper::body::to_bytes(_req.into_body()).await
}

async fn index(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
	let fresp = PREV_RESP.with(|resp| resp.borrow().clone());
	if fresp != ptr::null_mut() {
		unsafe {
            ffead_cpp_resp_cleanup(fresp);
        }
	}
    let mut request = ffead_request::default();
    let mut phrheaders = [phr_header::default(); 100];
    request.server_str = std::ffi::CString::new(SERVER_NAME).unwrap().into_raw();
    request.server_str_len = SERVER_NAME.len();
    request.method = std::ffi::CString::new(_req.method().as_str()).unwrap().into_raw();
    request.method_len = _req.method().as_str().len();
    request.path = std::ffi::CString::new(_req.uri().path()).unwrap().into_raw();
    request.path_len = _req.uri().path().len();
    request.version = if _req.version() == Version::HTTP_10 { 0 } else { 1 };
    
    let headers = _req.headers();
    let mut num_headers = 0;
    for (k, v) in headers {
        phrheaders[num_headers].name = std::ffi::CString::new(k.as_str()).unwrap().into_raw();
        phrheaders[num_headers].name_len = k.as_str().len();
        phrheaders[num_headers].value = std::ffi::CString::new(v.to_str().unwrap()).unwrap().into_raw();
        phrheaders[num_headers].value_len = v.to_str().unwrap().len();
        num_headers = num_headers + 1;
    }
    request.headers = phrheaders.as_mut_ptr();
    request.headers_len = num_headers as c_int;
    let body = executor::block_on(to_bytes(_req)).unwrap();
    request.body_len = body.to_vec().len();
    unsafe {
        request.body = std::ffi::CString::from_vec_unchecked(body.to_vec()).into_raw();
    }

    let mut response = FfeadResponse::default();
    let fresp: *mut c_void;
    unsafe {
        fresp = ffead_cpp_handle_rust_1(&request, &mut response.status_code, &mut response.url, &mut response.url_len, 
            response.headers.as_mut_ptr() , &mut response.headers_len, &mut response.body, &mut response.body_len);
    }

    //Cleanup all raw pointers passed to C
    unsafe {
        let _ = std::ffi::CString::from_raw(request.server_str);
        let _ = std::ffi::CString::from_raw(request.method);
        let _ = std::ffi::CString::from_raw(request.path);
        let _ = std::ffi::CString::from_raw(request.body);
        for i in 0..num_headers {
            let _ = std::ffi::CString::from_raw(phrheaders[i].name);
            let _ = std::ffi::CString::from_raw(phrheaders[i].value);
        }
    }
    
	PREV_RESP.with(|resp| {
        *resp.borrow_mut() = fresp
    });
    
    if response.status_code > 0 {
        let mut resp_build = Response::builder().status(response.status_code as u16);
        let resp_headers_len = response.headers_len;
        let rheaders = resp_build.headers_mut().unwrap();
        for i in 0..resp_headers_len as usize {
            let k = slice_from_raw(response.headers[i].name, response.headers[i].name_len);
            let v = slice_from_raw(response.headers[i].value, response.headers[i].value_len);
            rheaders.insert(std::str::from_utf8(k).unwrap(), HeaderValue::from_static(std::str::from_utf8(v).unwrap()));
        }
        rheaders.insert(SERVER, HeaderValue::from_static(SERVER_NAME));
        let body = slice_from_raw(response.body, response.body_len);
        
        Ok(resp_build.body(Body::from(body)).unwrap())
    } else {
        let url_path = slice_from_raw(response.url, response.url_len);
        let url_path = std::str::from_utf8(url_path).unwrap();
        if Path::new(url_path).exists() {
            let data = fs::read(url_path).unwrap();
            let body = Body::from(data);
            return Ok(Response::new(body));
        } else {
            Ok(not_found())
        }
    }
}

thread_local! {
    pub static PREV_RESP: RefCell<*mut c_void> = RefCell::new(ptr::null_mut());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();

    let arg0 = &args[1];
    let arg1 = &args[2];

    let c_server_directory = std::ffi::CString::new(arg0.as_str()).unwrap();

    println!("Bootstrapping ffead-cpp start...");
    unsafe {
        ffead_cpp_bootstrap(c_server_directory.as_ptr(), arg0.len(), 12);
    }
	println!("Bootstrapping ffead-cpp end...");

	println!("Initializing ffead-cpp start...");
    unsafe {
        ffead_cpp_init();
    }
	println!("Initializing ffead-cpp end...");

    let addr_str = format!("{}:{}", "0.0.0.0", arg1);

    let make_svc = make_service_fn(|_| {
        async {
            Ok::<_, hyper::Error>(service_fn(index))
        }
    });

    let addr = addr_str.parse().unwrap();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    println!("Cleaning up ffead-cpp start...");
    unsafe {
        ffead_cpp_cleanup();
    }
    println!("Cleaning up ffead-cpp end...");

    Ok(())
}