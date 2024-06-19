use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
        },
        render_asset::RenderAssets,
        render_graph::{
            self, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode,
            ViewNodeRunner,
        },
        render_phase::TrackedRenderPass,
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{BevyDefault, CachedTexture, TextureCache},
        view::ViewTarget,
        RenderApp,
    },
};

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct VordieLightSettings {
    pub setting: f32,
}

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct Params {
    pub offset: f32,
}

#[derive(Resource)]
struct VordieLightPipeline {
    sampler: Sampler,
    init_bind_group_layout: BindGroupLayout,
    main_bind_group_layout: BindGroupLayout,
    init_pipeline_id: CachedRenderPipelineId,
    main_pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for VordieLightPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap().clone();

        let init_bind_group_layout = render_device.create_bind_group_layout(
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
                    // Jumpflood params
                    uniform_buffer::<Params>(false),
                ),
            ),
        );
        let main_bind_group_layout = render_device.create_bind_group_layout(
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

        let assets_server = world.resource::<AssetServer>();
        let init_shader = assets_server.load("shaders/vordie_init.wgsl");
        let main_shader = assets_server.load("shaders/vordie_main.wgsl");

        let pipeline_cache = world.get_resource::<PipelineCache>().unwrap();
        let init_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_init_pipeline".into()),
            layout: vec![init_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: init_shader.clone(),
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
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        });
        let main_cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_main_pipeline".into()),
            layout: vec![main_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: main_shader.clone(),
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
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        });

        // We can create the sampler here since it won't change at runtime and doesn't depend on the view.
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        Self {
            sampler,
            init_bind_group_layout,
            main_bind_group_layout,
            init_pipeline_id: init_cached,
            main_pipeline_id: main_cached,
        }
    }
}

#[derive(Default)]
struct VordieNode;

impl ViewNode for VordieNode {
    // This query will only run on the view entity
    type ViewQuery = (
        &'static ViewTarget,
        // This makes sure the node only runs on cameras with the VordieLightSettings component
        &'static VordieLightSettings,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _vordie_light_settings): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let vordie_pipeline = world.resource::<VordieLightPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(init_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.init_pipeline_id)
        else {
            return Ok(());
        };
        let Some(main_pipeline) =
            pipeline_cache.get_render_pipeline(vordie_pipeline.main_pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<VordieLightSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // First pass: Initialize the jump flood algorithm
        {
            let view_texture = view_target.post_process_write();

            let bind_group = render_context.render_device().create_bind_group(
                "post_process_bind_group",
                &vordie_pipeline.init_bind_group_layout,
                &BindGroupEntries::sequential((
                    // Make sure to use the source view
                    view_texture.source,
                    // Use the sampler created for the pipeline
                    &vordie_pipeline.sampler,
                    // Set the settings binding, including the offset
                    settings_binding.clone(),
                    // Create new params binding
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
            render_pass.set_render_pipeline(init_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        // Begining the jump flood algorithm loop
        // Buffer for params in the jumpflood algorithm
        let render_device = world.get_resource::<RenderDevice>().unwrap().clone();
        let render_queue = world.resource::<RenderQueue>();

        let prev_texture_descriptor = TextureDescriptor {
            label: Some("jfa_source_texture"),
            size: Extent3d {
                width: view_target.main_texture().width(),
                height: view_target.main_texture().height(),
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

        // Temp fixed screen size
        let screen_size = Vec2::new(1024., 1024.);

        let passes = screen_size.x.log2().ceil() as i32;
        // let passes = 0;
        for i in 0..passes {
            // Create the destination textures
            let destination_texture_descriptor = TextureDescriptor {
                label: Some("jfa_destination_texture"),
                size: Extent3d {
                    width: view_target.main_texture().width(),
                    height: view_target.main_texture().height(),
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

            let mut params_buffer = UniformBuffer::<Params>::from(Params { offset });
            params_buffer.write_buffer(&render_device, render_queue);

            let bind_group = render_context.render_device().create_bind_group(
                "post_process_bind_group",
                &vordie_pipeline.main_bind_group_layout,
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

            let color_attachment = if i == passes - 1 {
                view_target.get_unsampled_color_attachment()
            } else {
                RenderPassColorAttachment {
                    view: &destination_view,
                    resolve_target: None,
                    ops: Operations::default(),
                }
            };
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
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
        }
        // println!("VordieLight2D: ThisNode::run");

        Ok(())
    }
}

#[derive(Component)]
struct JFATexture {
    // First mip is half the screen resolution, successive mips are half the previous
    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    texture: CachedTexture,
    mip_count: u32,
}

impl JFATexture {
    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    fn view(&self, base_mip_level: u32) -> TextureView {
        self.texture.texture.create_view(&TextureViewDescriptor {
            base_mip_level,
            mip_level_count: Some(1u32),
            ..Default::default()
        })
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

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
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
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<VordieLightPipeline>();
    }
}

pub mod prelude;
