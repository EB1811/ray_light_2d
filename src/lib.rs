use bevy::{
    core::FrameCount,
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::{
            ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{BevyDefault, CachedTexture, TextureCache},
        view::{ExtractedView, ViewTarget},
        Render, RenderApp, RenderSet,
    },
};

// Testing by step
const STEP: i32 = 5;

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct VordieLightSettings {
    pub u_rays_per_pixel: i32,
    pub u_emission_multi: f32,
    pub u_max_raymarch_steps: i32,
    pub u_dist_mod: f32,
    pub u_emission_range: f32,
    pub u_emission_dropoff: f32,
}
impl Default for VordieLightSettings {
    fn default() -> Self {
        Self {
            u_rays_per_pixel: 8,
            u_emission_multi: 1.0,
            u_max_raymarch_steps: 64,
            u_dist_mod: 1.0,
            u_emission_range: 1.5,
            u_emission_dropoff: 1.5,
        }
    }
}

#[derive(Component)]
pub struct GlobalIHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
}

fn prepare_gi_history_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView), With<VordieLightSettings>>,
) {
    for (entity, camera, view) in &views {
        if let Some(physical_target_size) = camera.physical_target_size {
            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            texture_descriptor.label = Some("gi_history_1_texture");
            let history_1_texture = texture_cache.get(&render_device, texture_descriptor.clone());

            texture_descriptor.label = Some("gi_history_2_texture");
            let history_2_texture = texture_cache.get(&render_device, texture_descriptor);

            let textures = if frame_count.0 % 2 == 0 {
                GlobalIHistoryTextures {
                    write: history_1_texture,
                    read: history_2_texture,
                }
            } else {
                GlobalIHistoryTextures {
                    write: history_2_texture,
                    read: history_1_texture,
                }
            };

            commands.entity(entity).insert(textures);
        }
    }
}

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct Params {
    pub screen_pixel_size: Vec2,
    pub offset: f32,
}

#[derive(Resource)]
struct VordieLightPipeline {
    sampler: Sampler,
    emiters_occs_bind_group_layout: BindGroupLayout,
    seed_bind_group_layout: BindGroupLayout,
    jfa_bind_group_layout: BindGroupLayout,
    dis_field_bind_group_layout: BindGroupLayout,
    gi_raycast_bind_group_layout: BindGroupLayout,
    denoise_bind_group_layout: BindGroupLayout,

    emiters_occs_pipeline_id: CachedRenderPipelineId,
    seed_pipeline_id: CachedRenderPipelineId,
    jfa_pipeline_id: CachedRenderPipelineId,
    dis_field_pipeline_id: CachedRenderPipelineId,
    gi_raycast_pipeline_id: CachedRenderPipelineId,
    denoise_pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for VordieLightPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap().clone();

        let emiters_occs_bind_group_layout = render_device.create_bind_group_layout(
            "vordie_light_emiters_occs_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::NonFiltering),
                ),
            ),
        );
        let seed_bind_group_layout = render_device.create_bind_group_layout(
            "vordie_light_init_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::NonFiltering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<VordieLightSettings>(false),
                ),
            ),
        );
        let jfa_bind_group_layout = render_device.create_bind_group_layout(
            "vordie_light_main_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::NonFiltering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<VordieLightSettings>(false),
                    // Jumpflood params
                    uniform_buffer::<Params>(false),
                ),
            ),
        );
        let dis_field_bind_group_layout = render_device.create_bind_group_layout(
            "vordie_light_dis_field_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::NonFiltering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<VordieLightSettings>(false),
                ),
            ),
        );
        let gi_raycast_bind_group_layout = render_device.create_bind_group_layout(
            "vordie_light_gi_raycast_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::NonFiltering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<VordieLightSettings>(false),
                    // Emitter and occluder texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // GI History (read)
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // Time
                    uniform_buffer::<f32>(false),
                ),
            ),
        );
        let denoise_bind_group_layout = render_device.create_bind_group_layout(
            "vordie_light_denoise_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::NonFiltering),
                    // GI History (read)
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // The settings uniform that will control the effect
                    uniform_buffer::<VordieLightSettings>(false),
                    // Screen pixel size
                    uniform_buffer::<Params>(false),
                ),
            ),
        );

        let assets_server = world.resource::<AssetServer>();
        let emiters_occs_shader = assets_server.load("shaders/vordie_emiters_occs.wgsl");
        let seed_shader = assets_server.load("shaders/vordie_seed.wgsl");
        let jfa_shader = assets_server.load("shaders/vordie_jfa.wgsl");
        let dis_field_shader = assets_server.load("shaders/vordie_dis_field.wgsl");
        let gi_raycast_shader = assets_server.load("shaders/vordie_gi_raycast.wgsl");
        let denoise_shader = assets_server.load("shaders/vordie_denoise.wgsl");

        let pipeline_cache = world.get_resource::<PipelineCache>().unwrap();
        let emiters_occs_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_emiters_occs_pipeline".into()),
            layout: vec![emiters_occs_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: emiters_occs_shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        });
        let seed_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_seed_pipeline".into()),
            layout: vec![seed_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: seed_shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        });
        let jfa_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_jfa_pipeline".into()),
            layout: vec![jfa_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: jfa_shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        });
        let dis_field_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_dis_field_pipeline".into()),
            layout: vec![dis_field_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: dis_field_shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        });
        let gi_raycast_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_gi_raycast_pipeline".into()),
            layout: vec![gi_raycast_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: gi_raycast_shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        });
        let denoise_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_denoise_pipeline".into()),
            layout: vec![denoise_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: denoise_shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
        });

        // We can create the sampler here since it won't change at runtime and doesn't depend on the view.
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        Self {
            sampler,
            emiters_occs_bind_group_layout,
            seed_bind_group_layout,
            jfa_bind_group_layout,
            dis_field_bind_group_layout,
            gi_raycast_bind_group_layout,
            denoise_bind_group_layout,

            emiters_occs_pipeline_id: emiters_occs_cached,
            seed_pipeline_id: seed_cached,
            jfa_pipeline_id: jfa_cached,
            dis_field_pipeline_id: dis_field_cached,
            gi_raycast_pipeline_id: gi_raycast_cached,
            denoise_pipeline_id: denoise_cached,
        }
    }
}

#[derive(Default)]
struct VordieNode;

impl ViewNode for VordieNode {
    // This query will only run on the view entity
    type ViewQuery = (
        &'static ViewTarget,
        &'static GlobalIHistoryTextures,
        // This makes sure the node only runs on cameras with the VordieLightSettings component
        &'static VordieLightSettings,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, gi_history_textures, _vordie_light_settings): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let vordie_pipeline = world.resource::<VordieLightPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(emiters_occs_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.emiters_occs_pipeline_id)
        else {
            return Ok(());
        };
        let Some(seed_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.seed_pipeline_id)
        else {
            return Ok(());
        };
        let Some(main_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.jfa_pipeline_id)
        else {
            return Ok(());
        };
        let Some(dis_field_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.dis_field_pipeline_id)
        else {
            return Ok(());
        };
        let Some(gi_raycast_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.gi_raycast_pipeline_id)
        else {
            return Ok(());
        };
        let Some(denoise_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.denoise_pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<VordieLightSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // Creating emitters and occluders texture
        let emitters_occluders_descriptor = TextureDescriptor {
            label: Some("emitters_occluders_texture"),
            size: Extent3d {
                width: view_target.main_texture().width() / 2,
                height: view_target.main_texture().height() / 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let emitters_occluders_view = render_context
            .render_device()
            .create_texture(&emitters_occluders_descriptor)
            .create_view(&TextureViewDescriptor {
                ..Default::default()
            });
        {
            let view_texture = view_target.main_texture_view();

            let bind_group = render_context.render_device().create_bind_group(
                "emitters_occluders_bind_group",
                &vordie_pipeline.emiters_occs_bind_group_layout,
                &BindGroupEntries::sequential((
                    view_texture,
                    // Use the sampler created for the pipeline
                    &vordie_pipeline.sampler,
                )),
            );
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("emitters_occluders"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &emitters_occluders_view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(emiters_occs_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        if STEP == 0 {
            return Ok(());
        }

        // Initialize the jump flood algorithm
        {
            let view_texture = view_target.post_process_write();

            let bind_group = render_context.render_device().create_bind_group(
                "post_process_bind_group",
                &vordie_pipeline.seed_bind_group_layout,
                &BindGroupEntries::sequential((
                    // Make sure to use the source view
                    view_texture.source,
                    // Use the sampler created for the pipeline
                    &vordie_pipeline.sampler,
                    // Set the settings binding, including the offset
                    settings_binding.clone(),
                )),
            );
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("vordie_light_init"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: view_texture.destination,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(seed_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        if STEP == 1 {
            return Ok(());
        }

        // Begining the jump flood algorithm loop
        // Buffer for params in the jumpflood algorithm
        let render_device = world.get_resource::<RenderDevice>().unwrap().clone();
        let render_queue = world.resource::<RenderQueue>();
        {
            let prev_texture_descriptor = TextureDescriptor {
                label: Some("jfa_source_texture"),
                size: Extent3d {
                    width: view_target.main_texture().width() / 2,
                    height: view_target.main_texture().width() / 2,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            };
            let mut prev_view = render_context
                .render_device()
                .create_texture(&prev_texture_descriptor)
                .create_view(&TextureViewDescriptor {
                    ..Default::default()
                });

            let screen_size = Vec2::new(
                (view_target.main_texture().width() / 2) as f32,
                (view_target.main_texture().height() / 2) as f32,
            );

            let passes = f32::max(screen_size.x, screen_size.y).log2().ceil() as i32;
            // print!("passes: {}", passes);
            // let passes = 10;
            let stop_at = 50;

            for i in 0..=passes {
                // Create the destination textures
                let destination_texture_descriptor = TextureDescriptor {
                    label: Some("jfa_destination_texture"),
                    size: Extent3d {
                        width: view_target.main_texture().width() / 2,
                        height: view_target.main_texture().width() / 2,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba16Float,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                };
                let destination_view = render_context
                    .render_device()
                    .create_texture(&destination_texture_descriptor)
                    .create_view(&TextureViewDescriptor {
                        ..Default::default()
                    });

                let source = if i == 0 {
                    view_target.main_texture_view().clone()
                } else {
                    prev_view.clone()
                };

                let offset = 2f32.powi(passes - i - 1);

                let mut params_buffer = UniformBuffer::<Params>::from(Params {
                    screen_pixel_size: screen_size,
                    offset,
                });
                params_buffer.write_buffer(&render_device, render_queue);

                let bind_group = render_context.render_device().create_bind_group(
                    "post_process_bind_group",
                    &vordie_pipeline.jfa_bind_group_layout,
                    &BindGroupEntries::sequential((
                        // Make sure to use the source view
                        &source,
                        // Use the sampler created for the pipeline
                        &vordie_pipeline.sampler,
                        // Set the settings binding, including the offset
                        settings_binding.clone(),
                        // Create new params binding
                        params_buffer.binding().unwrap(),
                    )),
                );

                let color_attachment = if i == passes || i == stop_at {
                    view_target.get_unsampled_color_attachment()
                } else {
                    RenderPassColorAttachment {
                        view: &destination_view,
                        resolve_target: None,
                        ops: Operations::default(),
                    }
                };
                let mut render_pass =
                    render_context.begin_tracked_render_pass(RenderPassDescriptor {
                        label: Some("vordie_light_init"),
                        color_attachments: &[Some(color_attachment)],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                render_pass.set_render_pipeline(main_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.draw(0..3, 0..1);

                // Set the target for the next iteration
                prev_view = destination_view.clone();

                if i == stop_at {
                    break;
                }
            }
        }

        if STEP == 2 {
            return Ok(());
        }

        // Distance Field Pass
        {
            let view_texture = view_target.post_process_write();

            let bind_group = render_context.render_device().create_bind_group(
                "dis_field_bind_group",
                &vordie_pipeline.dis_field_bind_group_layout,
                &BindGroupEntries::sequential((
                    // Make sure to use the source view
                    view_texture.source,
                    // Use the sampler created for the pipeline
                    &vordie_pipeline.sampler,
                    // Set the settings binding, including the offset
                    settings_binding.clone(),
                )),
            );
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("vordie_light_init"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: view_texture.destination,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(dis_field_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        if STEP == 3 {
            return Ok(());
        }

        // GI Raycast Pass
        {
            let start1 = std::time::SystemTime::now();
            let since_the_epoch1 = start1
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            let mut time_buffer = UniformBuffer::<f32>::from(
                since_the_epoch1
                    .as_millis()
                    .to_string()
                    .chars()
                    .last()
                    .unwrap()
                    .to_digit(10)
                    .unwrap() as f32
                    + 1.0 / 5.0,
            );
            time_buffer.write_buffer(&render_device, render_queue);

            let view_texture = view_target.post_process_write();

            let bind_group = render_context.render_device().create_bind_group(
                "gi_raycast_bind_group",
                &vordie_pipeline.gi_raycast_bind_group_layout,
                &BindGroupEntries::sequential((
                    // Make sure to use the source view
                    view_texture.source,
                    // Use the sampler created for the pipeline
                    &vordie_pipeline.sampler,
                    // Set the settings binding, including the offset
                    settings_binding.clone(),
                    // Set the emitters and occluders texture
                    &emitters_occluders_view,
                    // Past frames
                    &gi_history_textures.read.default_view,
                    // Set the time
                    time_buffer.binding().unwrap(),
                )),
            );
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("vordie_light_init"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: view_texture.destination,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(gi_raycast_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        if STEP == 4 {
            return Ok(());
        }

        // Denoise Pass
        {
            let view_texture = view_target.post_process_write();

            let mut params_buffer = UniformBuffer::<Params>::from(Params {
                screen_pixel_size: Vec2::new(
                    (view_target.main_texture().width() / 2) as f32,
                    (view_target.main_texture().height() / 2) as f32,
                ),
                offset: 0.0,
            });
            params_buffer.write_buffer(&render_device, render_queue);

            let bind_group = render_context.render_device().create_bind_group(
                "denoise_bind_group",
                &vordie_pipeline.denoise_bind_group_layout,
                &BindGroupEntries::sequential((
                    // Make sure to use the source view
                    view_texture.source,
                    // Use the sampler created for the pipeline
                    &vordie_pipeline.sampler,
                    // Past frames
                    &gi_history_textures.read.default_view,
                    // Set the settings binding, including the offset
                    settings_binding.clone(),
                    // Set the params binding
                    params_buffer.binding().unwrap(),
                )),
            );
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("vordie_light_init"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: view_texture.destination,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                    Some(RenderPassColorAttachment {
                        view: &gi_history_textures.write.default_view,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(denoise_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

pub struct VordieLight2DPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct LightPass2DRenderLabel;

impl Plugin for VordieLight2DPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // The settings will be a component that lives in the main world but will
            // be extracted to the render world every frame.
            // This makes it possible to control the effect from the main world.
            // This plugin will take care of extracting it automatically.
            ExtractComponentPlugin::<VordieLightSettings>::default(),
            // The settings will also be the data used in the shader.
            // This plugin will prepare the component for the GPU by creating a uniform buffer
            // and writing the data to that buffer every frame.
            UniformComponentPlugin::<VordieLightSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                Render,
                prepare_gi_history_textures.in_set(RenderSet::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<VordieNode>>(Core2d, LightPass2DRenderLabel)
            .add_render_graph_edges(
                Core2d,
                // Specify the node ordering.
                // This will automatically create all required node edges to enforce the given ordering.
                (
                    Node2d::Tonemapping,
                    LightPass2DRenderLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<VordieLightPipeline>();
    }
}

pub mod prelude;
