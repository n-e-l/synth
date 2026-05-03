static const float TAU = 6.28318530717958647692;
static const float PI  = 3.14159265358979323846;

float2 audio(float t, float volume, float frequency, float4 options)
{
    float s = sin(t * frequency * TAU) * volume;
    return float2(s);
}
