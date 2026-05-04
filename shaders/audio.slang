import user_audio;

[[vk::binding(0, 0)]]
RWStructuredBuffer<float2> audio_data;

[[vk::push_constant]]
cbuffer PushConstants
{
    float4 options;
    float time;
    int samples_per_second;
    int total_samples;
    float frequency;
    float volume;
};

float3 palette(float t, float3 a, float3 b, float3 c, float3 d)
{
    return a + b * cos(6.28318 * (c * t + d));
}

[shader("compute")]
[numthreads(128, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    int i = int( id.x );
    if( i >= total_samples ) return;
    audio_data[i] = audio(time + float( i ) / samples_per_second, volume, frequency, options);
}
