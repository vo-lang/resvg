//! Vo wrapper for resvg — SVG rendering to PNG.
//!
//! # Features
//! - `native` (default): dynamic library for dlopen  
//! - `wasm`: compiled into the playground WASM binary
//! - `wasm-standalone`: pure C-ABI cdylib for dynamic WASM loading

use resvg::{usvg, tiny_skia};

fn render_svg_to_png(svg_str: &str) -> Result<Vec<u8>, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_str, &opt)
        .map_err(|e| e.to_string())?;
    let int_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(int_size.width(), int_size.height())
        .ok_or_else(|| "failed to allocate pixmap: SVG has zero size".to_string())?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

// ── Vo extension ABI (native + wasm-integrated) ───────────────────────────────

#[cfg(any(feature = "native", feature = "wasm"))]
mod vo_ext_impl {
    use super::render_svg_to_png;
    use vo_ext::prelude::*;
    use vo_runtime::builtins::error_helper::{write_error_to, write_nil_error};

    #[vo_fn("resvg", "Render")]
    pub fn render(call: &mut ExternCallContext) -> ExternResult {
        let svg = call.arg_str(0).to_string();
        match render_svg_to_png(&svg) {
            Ok(png_bytes) => {
                let slice_ref = call.alloc_bytes(&png_bytes);
                call.ret_ref(0, slice_ref);
                write_nil_error(call, 1);
            }
            Err(msg) => {
                call.ret_nil(0);
                write_error_to(call, 1, &msg);
            }
        }
        ExternResult::Ok
    }
}

#[cfg(feature = "native")]
vo_ext::export_extensions!();

#[cfg(feature = "wasm")]
vo_ext::export_extensions!(__STDLIB_github_com_vo_lang_resvg_Render);

/// Register resvg extern functions (wasm-integrated feature only).
#[cfg(feature = "wasm")]
pub fn register_externs(
    registry: &mut vo_runtime::ffi::ExternRegistry,
    externs: &[vo_runtime::bytecode::ExternDef],
) {
    for entry in VO_EXT_ENTRIES {
        for (id, def) in externs.iter().enumerate() {
            if def.name == entry.name {
                registry.register(id as u32, entry.func);
                break;
            }
        }
    }
}

// ── Standalone C-ABI WASM exports (wasm-standalone feature) ──────────────────
//
// Used when the .wasm binary is pre-built and dynamically loaded by the browser.
// Follows the Vo ext module v2 ABI:
//   vo_alloc / vo_dealloc  — memory management
//   github_com_vo_lang_resvg_Render — matches the Vo extern name exactly
//
// Input (v2 Bytes slot):  [u32 LE len][len bytes of UTF-8 SVG]
// Output (v2 tagged):     TAG_BYTES(0xE3)+[u32 len]+[png bytes]+TAG_NIL_ERROR(0xE0)
//                      or TAG_NIL_REF(0xE4)+TAG_ERROR_STR(0xE1)+[u16 len]+[msg bytes]

#[cfg(feature = "wasm-standalone")]
mod standalone {
    use super::render_svg_to_png;

    #[no_mangle]
    pub extern "C" fn vo_alloc(size: u32) -> *mut u8 {
        let mut buf = Vec::<u8>::with_capacity(size as usize);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf);
        ptr
    }

    #[no_mangle]
    pub extern "C" fn vo_dealloc(ptr: *mut u8, size: u32) {
        unsafe { drop(Vec::from_raw_parts(ptr, size as usize, size as usize)) };
    }

    /// Vo ext v2 ABI: decode [u32-len][svg-bytes] input, return tagged binary output.
    #[no_mangle]
    #[allow(non_snake_case)]
    pub extern "C" fn github_com_vo_lang_resvg_Render(
        input_ptr: *const u8,
        input_len: u32,
        out_len: *mut u32,
    ) -> *mut u8 {
        let output = unsafe {
            let input = std::slice::from_raw_parts(input_ptr, input_len as usize);
            render_v2(input)
        };
        let len = output.len() as u32;
        let mut boxed = output.into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        unsafe { *out_len = len; }
        ptr
    }

    fn render_v2(input: &[u8]) -> Vec<u8> {
        if input.len() < 4 {
            return error_out("resvg: input too short");
        }
        let svg_len = u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as usize;
        if input.len() < 4 + svg_len {
            return error_out("resvg: input truncated");
        }
        let svg_bytes = &input[4..4 + svg_len];
        let svg = match std::str::from_utf8(svg_bytes) {
            Ok(s) => s,
            Err(e) => return error_out(&format!("resvg: invalid utf-8: {}", e)),
        };
        match render_svg_to_png(svg) {
            Ok(png) => ok_out(&png),
            Err(e) => error_out(&e),
        }
    }

    fn ok_out(data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 4 + data.len() + 1);
        buf.push(0xE3u8);
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(data);
        buf.push(0xE0u8);
        buf
    }

    fn error_out(msg: &str) -> Vec<u8> {
        let bytes = msg.as_bytes();
        let len = bytes.len().min(0xFFFF);
        let mut buf = Vec::with_capacity(1 + 1 + 2 + len);
        buf.push(0xE4u8);
        buf.push(0xE1u8);
        buf.extend_from_slice(&(len as u16).to_le_bytes());
        buf.extend_from_slice(&bytes[..len]);
        buf
    }
}
