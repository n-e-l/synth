float2 audio(float t, float volume, float frequency, float4 options)
{
    float s = sin(t * frequency * 6.28318530) * volume;
    return float2(s);
}
