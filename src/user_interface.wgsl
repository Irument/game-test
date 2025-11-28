struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.uv = input.uv;
    output.clip_position = vec4<f32>(input.position,1.0, 1.0);
    output.color = input.color;
    return output;
}

@group(0) @binding(0)
var texture_view: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;
 
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var sample_color = textureSample(texture_view, texture_sampler, input.uv);
    return sample_color * input.color;
}


