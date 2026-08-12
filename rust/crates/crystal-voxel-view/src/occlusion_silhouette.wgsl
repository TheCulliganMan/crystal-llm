#import bevy_pbr::forward_io::VertexOutput

struct SilhouetteMaterial {
    color: vec4<f32>,
}

@group(2) @binding(0) var silhouette_texture: texture_2d<f32>;
@group(2) @binding(1) var silhouette_sampler: sampler;
@group(2) @binding(2) var<uniform> material: SilhouetteMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(silhouette_texture, silhouette_sampler, in.uv);
    if source.a < 0.5 {
        discard;
    }
    return vec4<f32>(source.rgb * material.color.rgb, source.a * material.color.a);
}
