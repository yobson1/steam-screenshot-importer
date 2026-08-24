use std::sync::mpsc::{Receiver, TryRecvError};

use anyhow::{Context as _, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use image::RgbaImage;

pub const OUTPUT_WIDTH: u32 = 250;
pub const OUTPUT_HEIGHT: u32 = 366;
pub const OUTPUT_WIDTH_F32: f32 = 250.0;
pub const OUTPUT_HEIGHT_F32: f32 = 366.0;

const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const SAMPLE_COUNT: u32 = 4;
const READBACK_SLOT_COUNT: usize = 3;

const SHADER: &str = r"
struct Corner {
    position_and_w: vec4<f32>,
}

struct QuadParams {
    corners: array<Corner, 4>,
}

@group(0) @binding(0) var artwork: texture_2d<f32>;
@group(0) @binding(1) var artwork_sampler: sampler;
@group(0) @binding(2) var<uniform> quad: QuadParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_projected(@builtin(vertex_index) vertex_id: u32) -> VertexOutput {
    var corner_id = vertex_id;
    var uv = vec2<f32>(0.0, 0.0);
    if (vertex_id == 1u) {
        uv = vec2<f32>(1.0, 0.0);
    } else if (vertex_id == 2u) {
        uv = vec2<f32>(0.0, 1.0);
    } else if (vertex_id == 3u) {
        uv = vec2<f32>(1.0, 1.0);
    }

    let corner = quad.corners[corner_id].position_and_w;
    var out: VertexOutput;
    // Multiplying XY by W keeps the requested screen-space position after the
    // perspective divide, while W gives us perspective-correct UV interpolation.
    out.position = vec4<f32>(corner.xy * corner.z, 0.0, corner.z);
    out.uv = uv;
    return out;
}

@fragment
fn fs_projected(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(artwork, artwork_sampler, input.uv);
}
";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedVertex {
    pub x: f32,
    pub y: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadParams {
    corners: [[f32; 4]; 4],
}

struct ArtworkTexture {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderTag {
    pub card_index: usize,
    pub generation: u64,
}

pub struct CompletedRender {
    pub tag: RenderTag,
    pub pixels: RgbaImage,
}

struct PendingReadback {
    tag: RenderTag,
    receiver: Receiver<Result<(), wgpu::BufferAsyncError>>,
}

struct ReadbackSlot {
    buffer: wgpu::Buffer,
    pending: Option<PendingReadback>,
}

pub struct OffscreenRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    _msaa_texture: wgpu::Texture,
    msaa_view: wgpu::TextureView,
    readback_slots: Vec<ReadbackSlot>,
    padded_bytes_per_row: u32,
    artwork: Vec<ArtworkTexture>,
}

impl OffscreenRenderer {
    #[allow(
        clippy::too_many_lines,
        reason = "WGPU resource construction is clearer when kept in initialization order"
    )]
    pub fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .context("no WGPU adapter is available for offscreen card rendering")?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("card-offscreen-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .context("failed to create the offscreen WGPU device")?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("projected-card-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("projected-card-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("projected-card-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("projected-card-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_projected"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_projected"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OUTPUT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("projected-card-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("projected-card-uniforms"),
            size: std::mem::size_of::<QuadParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_size = wgpu::Extent3d {
            width: OUTPUT_WIDTH,
            height: OUTPUT_HEIGHT,
            depth_or_array_layers: 1,
        };
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("projected-card-output"),
            size: output_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("projected-card-msaa"),
            size: output_size,
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let unpadded_bytes_per_row = OUTPUT_WIDTH * 4;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(256) * 256;
        let readback_slots = (0..READBACK_SLOT_COUNT)
            .map(|_| ReadbackSlot {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("projected-card-readback"),
                    size: u64::from(padded_bytes_per_row) * u64::from(OUTPUT_HEIGHT),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                pending: None,
            })
            .collect();

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            sampler,
            uniform_buffer,
            output_texture,
            output_view,
            _msaa_texture: msaa_texture,
            msaa_view,
            readback_slots,
            padded_bytes_per_row,
            artwork: Vec::new(),
        })
    }

    pub fn upload_artwork(&mut self, pixels: &RgbaImage) -> usize {
        let size = wgpu::Extent3d {
            width: pixels.width(),
            height: pixels.height(),
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("card-artwork"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pixels.width() * 4),
                rows_per_image: Some(pixels.height()),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("projected-card-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });
        self.artwork.push(ArtworkTexture {
            _texture: texture,
            bind_group,
        });
        self.artwork.len() - 1
    }

    /// Enqueues a render and returns immediately. A `false` result means all readback
    /// slots are busy; callers should retain only their latest desired projection and retry.
    pub fn submit(
        &mut self,
        artwork: usize,
        vertices: [ProjectedVertex; 4],
        tag: RenderTag,
    ) -> bool {
        let Some(slot_index) = self
            .readback_slots
            .iter()
            .position(|slot| slot.pending.is_none())
        else {
            return false;
        };

        let corners = vertices.map(|vertex| {
            let ndc_x = vertex.x / OUTPUT_WIDTH_F32 * 2.0 - 1.0;
            let ndc_y = 1.0 - vertex.y / OUTPUT_HEIGHT_F32 * 2.0;
            [ndc_x, ndc_y, vertex.w, 0.0]
        });
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&QuadParams { corners }),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("projected-card-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("projected-card-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&self.output_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.artwork[artwork].bind_group, &[]);
            pass.draw(0..4, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_slots[slot_index].buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(OUTPUT_HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: OUTPUT_WIDTH,
                height: OUTPUT_HEIGHT,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.readback_slots[slot_index].buffer.slice(..).map_async(
            wgpu::MapMode::Read,
            move |result| {
                let _ = sender.send(result);
            },
        );
        self.readback_slots[slot_index].pending = Some(PendingReadback { tag, receiver });
        true
    }

    pub fn poll_completed(&mut self) -> Result<Vec<CompletedRender>> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| anyhow!("failed while polling card readbacks: {error:?}"))?;

        let mut completed = Vec::new();
        for slot in &mut self.readback_slots {
            let status = match slot.pending.as_ref() {
                Some(pending) => pending.receiver.try_recv(),
                None => continue,
            };
            match status {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    slot.pending = None;
                    return Err(error).context("failed to map card readback buffer");
                }
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => {
                    slot.pending = None;
                    return Err(anyhow!("card readback callback was dropped"));
                }
            }

            let pending = slot.pending.take().expect("pending readback disappeared");
            let mapped = slot.buffer.slice(..).get_mapped_range();
            let pixels = read_straight_alpha_pixels(&mapped, self.padded_bytes_per_row)?;
            drop(mapped);
            slot.buffer.unmap();
            completed.push(CompletedRender {
                tag: pending.tag,
                pixels,
            });
        }
        Ok(completed)
    }

    pub fn has_pending_work(&self) -> bool {
        self.readback_slots
            .iter()
            .any(|slot| slot.pending.is_some())
    }
}

fn read_straight_alpha_pixels(mapped: &[u8], padded_bytes_per_row: u32) -> Result<RgbaImage> {
    let unpadded_bytes_per_row = (OUTPUT_WIDTH * 4) as usize;
    let mut pixels = Vec::with_capacity(unpadded_bytes_per_row * OUTPUT_HEIGHT as usize);
    for row in mapped.chunks_exact(padded_bytes_per_row as usize) {
        pixels.extend_from_slice(&row[..unpadded_bytes_per_row]);
    }

    // Resolving the MSAA target averages transparent and covered samples, yielding
    // premultiplied edge pixels. RenderImage expects straight-alpha BGRA, so undo
    // that multiplication before GPUI composites the image a second time.
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        if alpha > 0 && alpha < 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }

    RgbaImage::from_raw(OUTPUT_WIDTH, OUTPUT_HEIGHT, pixels)
        .context("WGPU returned an invalid card image buffer")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readback_removes_row_padding_and_unpremultiplies_msaa_edges() {
        let padded_bytes_per_row = (OUTPUT_WIDTH * 4).div_ceil(256) * 256;
        let mut mapped = vec![0; padded_bytes_per_row as usize * OUTPUT_HEIGHT as usize];
        mapped[..4].copy_from_slice(&[25, 50, 75, 128]);

        let pixels = read_straight_alpha_pixels(&mapped, padded_bytes_per_row).unwrap();

        assert_eq!(pixels.dimensions(), (OUTPUT_WIDTH, OUTPUT_HEIGHT));
        assert_eq!(pixels.get_pixel(0, 0).0, [50, 100, 149, 128]);
        assert_eq!(pixels.get_pixel(0, 1).0, [0, 0, 0, 0]);
    }
}
