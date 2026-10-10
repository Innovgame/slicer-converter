use criterion::{black_box, criterion_group, criterion_main, Criterion};
use slicer_converter_core::formats::anycubic::codecs::{pw0_rle, pws_rle};
use slicer_converter_core::formats::{ctb, goo};

fn make_pixels(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let x = (index % width) as u8;
        let y = (index / width) as u8;
        *pixel = x.wrapping_mul(3) ^ y.wrapping_mul(5);
    }
    pixels
}

fn bench_ctb_rle_decode(criterion: &mut Criterion) {
    let width = 512u32;
    let height = 512u32;
    let pixels = make_pixels(width as usize, height as usize);
    let encoded =
        ctb::encode_layer_data(&pixels, 0, width, height).expect("encode CTB sample layer");

    criterion.bench_function("ctb_rle_decode", |bencher| {
        bencher.iter(|| {
            ctb::decode_layer_data(black_box(&encoded), 0, width, height, 1, 0)
                .expect("decode CTB sample layer")
        })
    });
}

fn bench_goo_rle_encode(criterion: &mut Criterion) {
    let width = 512usize;
    let height = 512usize;
    let pixels = make_pixels(width, height);

    criterion.bench_function("goo_rle_encode", |bencher| {
        bencher.iter(|| {
            goo::encode_layer_data(black_box(&pixels), width as u32, height as u32)
                .expect("encode GOO layer")
        })
    });
}

fn bench_crc32(criterion: &mut Criterion) {
    let bytes = make_pixels(2048, 2048);

    criterion.bench_function("crc32_bytes", |bencher| {
        bencher.iter(|| crc32fast::hash(black_box(&bytes)))
    });
}

fn bench_ctb_to_goo_transcode_alloc_vs_reuse(criterion: &mut Criterion) {
    let width = 512u32;
    let height = 512u32;
    let pixels = make_pixels(width as usize, height as usize);
    let encoded =
        ctb::encode_layer_data(&pixels, 0, width, height).expect("encode CTB sample layer");

    criterion.bench_function("ctb_to_goo_transcode_alloc", |bencher| {
        bencher.iter(|| {
            let decoded = ctb::decode_layer_data(black_box(&encoded), 0, width, height, 1, 0)
                .expect("decode CTB sample layer");
            goo::encode_layer_data(black_box(&decoded), width, height)
                .expect("encode GOO sample layer")
        })
    });

    criterion.bench_function("ctb_to_goo_transcode_reuse", |bencher| {
        let mut buf = Vec::new();
        bencher.iter(|| {
            buf.clear();
            ctb::decode_layer_data_into(black_box(&encoded), 0, width, height, 1, 0, &mut buf)
                .expect("decode CTB sample layer into reusable buffer");
            goo::encode_layer_data(black_box(&buf), width, height).expect("encode GOO sample layer")
        })
    });
}

/// Benchmark Anycubic PW0 RLE decode: alloc vs buffer-reuse at large resolution.
/// Uses 11520×5120 (Photon Mono M5 resolution) to show worst-case alloc overhead.
fn bench_pw0_decode_alloc_vs_reuse(criterion: &mut Criterion) {
    let width = 11520usize;
    let height = 5120usize;
    let pixels = make_pixels(width, height);
    let encoded = pw0_rle::encode_pw0_rle4(&pixels);

    criterion.bench_function("pw0_decode_11520x5120_alloc", |bencher| {
        bencher.iter(|| pw0_rle::decode_pw0_rle4(black_box(&encoded)).expect("decode PW0"))
    });

    criterion.bench_function("pw0_decode_11520x5120_reuse", |bencher| {
        let mut buf = Vec::with_capacity(width * height);
        bencher.iter(|| {
            pw0_rle::decode_pw0_rle4_into(black_box(&encoded), &mut buf)
                .expect("decode PW0 into buf")
        })
    });
}

/// Benchmark Anycubic PWS RLE decode: alloc vs buffer-reuse at large resolution.
fn bench_pws_decode_alloc_vs_reuse(criterion: &mut Criterion) {
    let width = 1440usize;
    let height = 2560usize;
    let pixels = make_pixels(width, height);
    let encoded = pws_rle::encode_pws_rle1(&pixels);

    criterion.bench_function("pws_decode_1440x2560_alloc", |bencher| {
        bencher.iter(|| pws_rle::decode_pws_rle1(black_box(&encoded)).expect("decode PWS"))
    });

    criterion.bench_function("pws_decode_1440x2560_reuse", |bencher| {
        let mut buf = Vec::with_capacity(width * height);
        bencher.iter(|| {
            pws_rle::decode_pws_rle1_into(black_box(&encoded), &mut buf)
                .expect("decode PWS into buf")
        })
    });
}

/// Benchmark GOO RLE decode: alloc vs buffer-reuse at large resolution.
fn bench_goo_decode_alloc_vs_reuse(criterion: &mut Criterion) {
    let width = 11520u32;
    let height = 5120u32;
    let pixels = make_pixels(width as usize, height as usize);
    let encoded = goo::encode_layer_data(&pixels, width, height).expect("encode GOO");

    criterion.bench_function("goo_decode_11520x5120_alloc", |bencher| {
        bencher.iter(|| {
            goo::decode_layer_data(black_box(&encoded), width, height).expect("decode GOO")
        })
    });

    criterion.bench_function("goo_decode_11520x5120_reuse", |bencher| {
        let mut buf = Vec::with_capacity((width * height) as usize);
        bencher.iter(|| {
            goo::rle::decode_goo_rle_into(black_box(&encoded), width, height, &mut buf)
                .expect("decode GOO into buf")
        })
    });
}

/// Benchmark CTB RLE encode: alloc vs buffer-reuse at large resolution.
fn bench_ctb_encode_alloc_vs_reuse(criterion: &mut Criterion) {
    let width = 11520usize;
    let height = 5120usize;
    let pixels = make_pixels(width, height);

    criterion.bench_function("ctb_encode_11520x5120_alloc", |bencher| {
        bencher.iter(|| ctb::rle::encode_ctb_rle(black_box(&pixels)).expect("encode CTB"))
    });

    criterion.bench_function("ctb_encode_11520x5120_reuse", |bencher| {
        let mut buf = Vec::with_capacity(width * height / 2);
        bencher.iter(|| {
            ctb::rle::encode_ctb_rle_into(black_box(&pixels), &mut buf)
                .expect("encode CTB into buf")
        })
    });
}

/// Benchmark GOO RLE encode: alloc vs buffer-reuse at large resolution.
fn bench_goo_encode_alloc_vs_reuse(criterion: &mut Criterion) {
    let width = 11520u32;
    let height = 5120u32;
    let pixels = make_pixels(width as usize, height as usize);

    criterion.bench_function("goo_encode_11520x5120_alloc", |bencher| {
        bencher
            .iter(|| goo::encode_layer_data(black_box(&pixels), width, height).expect("encode GOO"))
    });

    criterion.bench_function("goo_encode_11520x5120_reuse", |bencher| {
        let mut buf = Vec::with_capacity((width * height) as usize / 2);
        bencher.iter(|| {
            goo::rle::encode_goo_rle_into(black_box(&pixels), &mut buf)
                .expect("encode GOO into buf")
        })
    });
}

criterion_group!(
    codec_benches,
    bench_ctb_rle_decode,
    bench_goo_rle_encode,
    bench_crc32,
    bench_ctb_to_goo_transcode_alloc_vs_reuse,
    bench_pw0_decode_alloc_vs_reuse,
    bench_pws_decode_alloc_vs_reuse,
    bench_goo_decode_alloc_vs_reuse,
    bench_ctb_encode_alloc_vs_reuse,
    bench_goo_encode_alloc_vs_reuse
);
criterion_main!(codec_benches);
