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
        renderer::{RenderContext, RenderDevice},
        texture::{BevyDefault, TextureCache},
        view::ViewTarget,
        RenderApp,
    },
};

#[derive(Resource, Clone, Deref, AsBindGroup)]
struct VordieLightImage {
    texture: Handle<Image>,
}

#[derive(Resource)]
struct VordieLightPipeline {
    bind_group: BindGroup,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for VordieLightPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap().clone();
        let mut textures = world.get_resource_mut::<TextureCache>().unwrap();

        let v_output_desc = TextureDescriptor {
            label: Some("v_output"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::bevy_default(),
            view_formats: &[],
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        };
        let v_output = textures.get(&render_device, v_output_desc);

        let v_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("vordie_light_init"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            compare: None,
            ..Default::default()
        });
        let layout = render_device.create_bind_group_layout(
            "vordie_light_init_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let bind_group = render_device.create_bind_group(
            None,
            &layout,
            &BindGroupEntries::sequential((
                // Make sure to use the source view
                BindingResource::TextureView(&v_output.default_view),
                // Use the sampler created for the pipeline
                BindingResource::Sampler(&v_sampler),
            )),
        );

        let assets_server = world.resource::<AssetServer>();
        let shader = assets_server.load("shaders/vordie_init.wgsl");

        let pipeline_cache = world.get_resource::<PipelineCache>().unwrap();
        let cached = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("vordie_init_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: shader.clone(),
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

        Self {
            bind_group,
            pipeline_id: cached,
        }
    }
}

#[derive(Default)]
struct VordieNode;

impl ViewNode for VordieNode {
    type ViewQuery = &'static ViewTarget;

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let vordie_pipeline = world.resource::<VordieLightPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(vordie_pipeline.pipeline_id) else {
            return Ok(());
        };

        let view_texture = view_target.post_process_write();

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
        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &vordie_pipeline.bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        println!("VordieLight2D: ThisNode::run");

        Ok(())
    }
}

pub struct VordieLight2DPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct LightPass2DRenderLabel;

impl Plugin for VordieLight2DPlugin {
    fn build(&self, app: &mut App) {
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
