static const float TAU = 6.28318530717958647692;
static const float PI  = 3.14159265358979323846;

float2 audio(float t, float volume, float frequency, float4 options)
{
    float s1 = sin(options[0] * t * 31.5) * sin(options[2] * t * frequency * 1000 * TAU) * volume;
    float s2 = sin(options[1] * t * 31.5) * sin(options[3] * t * frequency * 1000 * TAU) * volume;
    return float2(s1, s2);
}
