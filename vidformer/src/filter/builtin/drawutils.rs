/// This module reimplements the drawutils.c file from FFmpeg.
/// We need to reimplement this file because it is not exposed in the FFmpeg Rust bindings.
///
/// The goal is to allow `ffi::` to be replaced with `drawutils::` and maintain API compatibility.
/// The implementation is based on the original C code from FFmpeg and bindgen and cleaned up with manual/ChatGPT updates.
use rusty_ffmpeg::ffi;

const MAX_PLANES: usize = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct FFDrawContext {
    pub(crate) desc: *const ffi::AVPixFmtDescriptor,
    pub(crate) format: ffi::AVPixelFormat,
    pub(crate) nb_planes: ::std::os::raw::c_uint,
    pub(crate) pixelstep: [::std::os::raw::c_int; MAX_PLANES],
    pub(crate) hsub: [u8; MAX_PLANES],
    pub(crate) vsub: [u8; MAX_PLANES],
    pub(crate) hsub_max: u8,
    pub(crate) vsub_max: u8,
    pub(crate) range: ffi::AVColorRange,
    pub(crate) flags: ::std::os::raw::c_uint,
    pub(crate) csp: ffi::AVColorSpace,
    pub(crate) rgb2yuv: [[f64; 3usize]; 3usize],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct FFDrawColor {
    pub(crate) rgba: [u8; 4usize],
    pub(crate) comp: [FFDrawColor__bindgen_ty_1; 4usize],
}
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) union FFDrawColor__bindgen_ty_1 {
    pub(crate) u32_: [u32; 4usize],
    pub(crate) u16_: [u16; 8usize],
    pub(crate) u8_: [u8; 16usize],
}

pub(crate) unsafe fn ff_draw_init(
    draw: *mut FFDrawContext,
    format: ffi::AVPixelFormat,
    flags: ::std::os::raw::c_uint,
) -> ::std::os::raw::c_int {
    ff_draw_init2(
        draw,
        format,
        ffi::AVColorSpace_AVCOL_SPC_UNSPECIFIED,
        ffi::AVColorRange_AVCOL_RANGE_UNSPECIFIED,
        flags,
    )
}

pub(crate) unsafe fn ff_draw_init2(
    draw: *mut FFDrawContext,
    format: ffi::AVPixelFormat,
    csp: ffi::AVColorSpace,
    range: ffi::AVColorRange,
    flags: ::std::os::raw::c_uint,
) -> ::std::os::raw::c_int {
    unsafe {
        let desc = ffi::av_pix_fmt_desc_get(format);
        if desc.is_null() || (*desc).name.is_null() {
            return ffi::AVERROR(ffi::EINVAL);
        }
        if (*desc).flags & ffi::AV_PIX_FMT_FLAG_BE as u64 != 0 {
            return ffi::AVERROR(ffi::ENOSYS);
        }
        if (*desc).flags
            & !(ffi::AV_PIX_FMT_FLAG_PLANAR | ffi::AV_PIX_FMT_FLAG_RGB | ffi::AV_PIX_FMT_FLAG_ALPHA)
                as u64
            != 0
        {
            return ffi::AVERROR(ffi::ENOSYS);
        }

        let mut csp = csp;
        if csp == ffi::AVColorSpace_AVCOL_SPC_UNSPECIFIED {
            csp = if (*desc).flags & ffi::AV_PIX_FMT_FLAG_RGB as u64 != 0 {
                ffi::AVColorSpace_AVCOL_SPC_RGB
            } else {
                ffi::AVColorSpace_AVCOL_SPC_SMPTE170M
            };
        }

        let luma = if (*desc).flags & ffi::AV_PIX_FMT_FLAG_RGB as u64 == 0 {
            ffi::av_csp_luma_coeffs_from_avcsp(csp)
        } else {
            std::ptr::null()
        };
        if luma.is_null() && (*desc).flags & ffi::AV_PIX_FMT_FLAG_RGB as u64 == 0 {
            return ffi::AVERROR(ffi::EINVAL);
        }

        let mut range = range;
        if range == ffi::AVColorRange_AVCOL_RANGE_UNSPECIFIED {
            range = match format {
                ffi::AVPixelFormat_AV_PIX_FMT_YUVJ420P
                | ffi::AVPixelFormat_AV_PIX_FMT_YUVJ422P
                | ffi::AVPixelFormat_AV_PIX_FMT_YUVJ444P
                | ffi::AVPixelFormat_AV_PIX_FMT_YUVJ411P
                | ffi::AVPixelFormat_AV_PIX_FMT_YUVJ440P
                | _ if csp == ffi::AVColorSpace_AVCOL_SPC_RGB => ffi::AVColorRange_AVCOL_RANGE_JPEG,
                _ => ffi::AVColorRange_AVCOL_RANGE_MPEG,
            };
        }

        if range != ffi::AVColorRange_AVCOL_RANGE_JPEG
            && range != ffi::AVColorRange_AVCOL_RANGE_MPEG
        {
            return ffi::AVERROR(ffi::EINVAL);
        }

        let mut nb_planes = 0;
        let mut pixelstep = [0i32; MAX_PLANES];
        let mut depthb = 0;

        for i in 0..(*desc).nb_components {
            let c = &(*desc).comp[i as usize];
            if c.depth < 8 || c.depth > 16 {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            if c.plane >= MAX_PLANES as i32 {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            if c.shift != 0 && (c.shift + c.depth) & 0x7 != 0 {
                return ffi::AVERROR(ffi::ENOSYS);
            }

            let db = (c.depth + 7) / 8;
            if depthb != 0 && depthb != db {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            depthb = db;
            if db * (c.offset + 1) as i32 > 16 {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            if c.offset as i32 % db != 0 {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            if pixelstep[c.plane as usize] != 0 && pixelstep[c.plane as usize] != c.step as i32 {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            pixelstep[c.plane as usize] = c.step as i32;
            if pixelstep[c.plane as usize] >= 8 {
                return ffi::AVERROR(ffi::ENOSYS);
            }
            nb_planes = nb_planes.max((c.plane + 1) as u32);
        }

        std::ptr::write(
            draw,
            FFDrawContext {
                desc,
                format,
                nb_planes,
                range,
                csp,
                flags,
                pixelstep,
                hsub: [(*desc).log2_chroma_w; 4],
                vsub: [(*desc).log2_chroma_h; 4],
                hsub_max: (*desc).log2_chroma_w,
                vsub_max: (*desc).log2_chroma_h,
                rgb2yuv: [[0.0; 3]; 3],
            },
        );

        if !luma.is_null() {
            ff_fill_rgb2yuv_table(luma.as_ref().unwrap(), &mut (*draw).rgb2yuv);
        }

        0
    }
}

fn ff_fill_rgb2yuv_table(coeffs: &ffi::AVLumaCoefficients, rgb2yuv: &mut [[f64; 3usize]; 3usize]) {
    let cr = ffi::av_q2d(coeffs.cr);
    let cg = ffi::av_q2d(coeffs.cg);
    let cb = ffi::av_q2d(coeffs.cb);

    // Inline the ycgco matrix
    if cr == 0.25 && cg == 0.5 && cb == 0.25 {
        *rgb2yuv = [[0.25, 0.5, 0.25], [-0.25, 0.5, -0.25], [0.5, 0.0, -0.5]];
        return;
    }
    // Inline the gbr matrix
    else if cr == 1.0 && cg == 1.0 && cb == 1.0 {
        *rgb2yuv = [[0.0, 1.0, 0.0], [0.0, -0.5, 0.5], [0.5, -0.5, 0.0]];
        return;
    }

    rgb2yuv[0][0] = cr;
    rgb2yuv[0][1] = cg;
    rgb2yuv[0][2] = cb;

    let bscale = 0.5 / (cb - 1.0);
    let rscale = 0.5 / (cr - 1.0);

    rgb2yuv[1][0] = bscale * cr;
    rgb2yuv[1][1] = bscale * cg;
    rgb2yuv[1][2] = 0.5;

    rgb2yuv[2][0] = 0.5;
    rgb2yuv[2][1] = rscale * cg;
    rgb2yuv[2][2] = rscale * cb;
}

pub(crate) unsafe fn ff_draw_color(
    draw: *mut FFDrawContext,
    color: *mut FFDrawColor,
    rgba: *const u8,
) {
    unsafe {
        let draw = &*draw;
        let color = &mut *color;
        let desc = &*draw.desc;

        let mut yuvad = [0.0; 4];
        let mut rgbad = [0.0; 4];

        // Copy rgba to color->rgba if they are not the same
        if rgba != color.rgba.as_ptr() {
            std::ptr::copy_nonoverlapping(rgba, color.rgba.as_mut_ptr(), 4);
        }

        // Initialize color->comp to zero
        std::ptr::write_bytes(color.comp.as_mut_ptr(), 0, color.comp.len());

        // Normalize RGBA values to the [0, 1] range
        for i in 0..4 {
            rgbad[i] = color.rgba[i] as f64 / 255.0;
        }

        // Convert RGB to YUV if necessary
        if desc.flags & ffi::AV_PIX_FMT_FLAG_RGB as u64 != 0 {
            yuvad[..3].copy_from_slice(&rgbad[..3]);
        } else {
            ff_matrix_mul_3x3_vec(&mut yuvad, &rgbad, &draw.rgb2yuv);
        }

        // Alpha channel
        yuvad[3] = rgbad[3];

        // Adjust YUV values according to the color range
        for i in 0..3 {
            let chroma = (desc.flags & ffi::AV_PIX_FMT_FLAG_RGB as u64 == 0) && i > 0;
            if draw.range == ffi::AVColorRange_AVCOL_RANGE_MPEG {
                yuvad[i] *= if chroma { 224.0 / 255.0 } else { 219.0 / 255.0 };
                yuvad[i] += if chroma { 128.0 / 255.0 } else { 16.0 / 255.0 };
            } else if chroma {
                yuvad[i] += 0.5;
            }
        }

        // Handle grayscale formats
        if desc.nb_components <= 2 {
            yuvad[1] = yuvad[3];
        }

        // Pack the YUV values into the color->comp array
        for i in 0..desc.nb_components as usize {
            let val = (yuvad[i] * ((1 << (desc.comp[i].depth + desc.comp[i].shift)) - 1) as f64
                + 0.5)
                .round() as u32;

            if desc.comp[i].depth > 8 {
                color.comp[desc.comp[i].plane as usize].u16_[desc.comp[i].offset as usize / 2] =
                    val as u16;
            } else {
                color.comp[desc.comp[i].plane as usize].u8_[desc.comp[i].offset as usize] =
                    val as u8;
            }
        }
    }
}

fn ff_matrix_mul_3x3_vec(dst: &mut [f64; 4], src: &[f64; 4], mat: &[[f64; 3usize]; 3usize]) {
    for i in 0..3 {
        dst[i] = mat[i][0] * src[0] + mat[i][1] * src[1] + mat[i][2] * src[2];
    }
}

fn pointer_at(
    draw: &FFDrawContext,
    data: &[*mut u8],
    linesize: &[::std::os::raw::c_int],
    plane: usize,
    x: ::std::os::raw::c_int,
    y: ::std::os::raw::c_int,
) -> *mut u8 {
    unsafe {
        data[plane].offset(
            ((y >> draw.vsub[plane]) * linesize[plane]
                + (x >> draw.hsub[plane]) * draw.pixelstep[plane]) as isize,
        )
    }
}

pub(crate) unsafe fn ff_copy_rectangle2(
    draw: *mut FFDrawContext,
    dst: *mut *mut u8,
    dst_linesize: *mut ::std::os::raw::c_int,
    src: *mut *mut u8,
    src_linesize: *mut ::std::os::raw::c_int,
    dst_x: ::std::os::raw::c_int,
    dst_y: ::std::os::raw::c_int,
    src_x: ::std::os::raw::c_int,
    src_y: ::std::os::raw::c_int,
    w: ::std::os::raw::c_int,
    h: ::std::os::raw::c_int,
) {
    unsafe {
        let draw = &*draw;
        let dst = std::slice::from_raw_parts_mut(dst, draw.nb_planes as usize);
        let dst_linesize = std::slice::from_raw_parts_mut(dst_linesize, draw.nb_planes as usize);
        let src = std::slice::from_raw_parts_mut(src, draw.nb_planes as usize);
        let src_linesize = std::slice::from_raw_parts_mut(src_linesize, draw.nb_planes as usize);

        for plane in 0..draw.nb_planes as usize {
            let mut p = pointer_at(draw, src, src_linesize, plane, src_x, src_y);
            let mut q = pointer_at(draw, dst, dst_linesize, plane, dst_x, dst_y);
            let wp =
                ((w + (1 << draw.hsub[plane]) - 1) >> draw.hsub[plane]) * draw.pixelstep[plane];
            let hp = (h + (1 << draw.vsub[plane]) - 1) >> draw.vsub[plane];

            for _ in 0..hp {
                std::ptr::copy_nonoverlapping(p, q, wp as usize);
                p = p.offset(src_linesize[plane] as isize);
                q = q.offset(dst_linesize[plane] as isize);
            }
        }
    }
}

pub(crate) unsafe fn ff_fill_rectangle(
    draw: *mut FFDrawContext,
    color: *mut FFDrawColor,
    dst: *mut *mut u8,
    dst_linesize: *mut ::std::os::raw::c_int,
    dst_x: ::std::os::raw::c_int,
    dst_y: ::std::os::raw::c_int,
    w: ::std::os::raw::c_int,
    h: ::std::os::raw::c_int,
) {
    unsafe {
        let draw = &*draw;
        let color = &mut *color;
        let dst = std::slice::from_raw_parts_mut(dst, draw.nb_planes as usize);
        let dst_linesize = std::slice::from_raw_parts_mut(dst_linesize, draw.nb_planes as usize);

        for plane in 0..draw.nb_planes as usize {
            let p0 = pointer_at(draw, dst, dst_linesize, plane, dst_x, dst_y);
            let wp = ((w + (1 << draw.hsub[plane]) - 1) >> draw.hsub[plane]) as usize;
            let hp = ((h + (1 << draw.vsub[plane]) - 1) >> draw.vsub[plane]) as usize;
            if hp == 0 {
                return;
            }
            let mut p = p0;

            // Handle big-endian systems with depth > 8
            if cfg!(target_endian = "big") && draw.desc.as_ref().unwrap().comp[0].depth > 8 {
                for x in 0..(draw.pixelstep[plane] / 2) as usize {
                    color.comp[plane].u16_[x] = color.comp[plane].u16_[x].swap_bytes();
                }
            }

            // Copy the first line from color
            for _ in 0..wp {
                std::ptr::copy_nonoverlapping(
                    color.comp[plane].u8_.as_ptr(),
                    p,
                    draw.pixelstep[plane] as usize,
                );
                p = p.offset(draw.pixelstep[plane] as isize);
            }

            // Copy the rest of the lines from the first line
            let wp_bytes = (wp * draw.pixelstep[plane] as usize) as isize;
            p = p0.offset(dst_linesize[plane] as isize);
            for _ in 1..hp {
                std::ptr::copy_nonoverlapping(p0, p, wp_bytes as usize);
                p = p.offset(dst_linesize[plane] as isize);
            }
        }
    }
}
