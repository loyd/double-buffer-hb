use double_buffer_hb::{Collector, DoubleBuffer, Writer};
use std::hint::black_box;

/// Monomorphized writer entry point; assembly symbol: `double_buffer_write`.
#[unsafe(export_name = "double_buffer_write")]
#[inline(never)]
pub fn write(writer: &mut Writer<'_, u64>, value: u64) {
    writer.write(|buffer| *buffer = value);
}

/// Monomorphized collector entry point; assembly symbol: `double_buffer_collect`.
#[unsafe(export_name = "double_buffer_collect")]
#[inline(never)]
pub fn collect(collector: &mut Collector<'_, u64>, replacement: u64) -> u64 {
    collector.collect(replacement)
}

fn main() {
    let mut buffer = DoubleBuffer::new(black_box([0_u64; 2]));
    let (mut writer, mut collector) = buffer.split();
    write(black_box(&mut writer), black_box(42));
    black_box(collect(black_box(&mut collector), black_box(0)));
}
