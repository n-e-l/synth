
// From https://github.com/0b5vr/wavenerd-dubplates/blob/main/shaders/20250712_drum_machine_tutorial.glsl
// All copyright goes to them

#define S2T (15.0 / bpm)
#define B2T (60.0 / bpm)
#define ZERO min(0, int(bpm))
#define saturate(x) clamp(x, 0., 1.)
#define linearstep(a,b,x) saturate(((x)-(a))/((b)-(a)))
#define clip(x) clamp(x, -1., 1.)
#define lofi(i,m) (floor((i)/(m))*(m))
#define u2b(u) ((u) * 2.0 - 1.0)
#define b2u(b) ((b) * 0.5 + 0.5)
#define tri(x) (1.0 - 4.0 * abs(fract((x) + 0.25) - 0.5))
#define repeat(i, n) for (int i = ZERO; i < n; i++)
#define p2f(i) (exp2(((i)-69.)/12.)*440.)

const float SWING = 0.5;

const float PI = acos(-1.0);
const float TAU = PI * 2.0;

uvec3 hash3u(uvec3 v) {
    v = v * 1145141919u + 1919810u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> 16u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return v;
}

vec2 cis(float t) {
    return vec2(cos(t), sin(t));
}

vec2 cheapnoise(float t) {
    uvec3 s=uvec3(t * 256.0);
    float p=fract(t * 256.0);

    vec3 dice;
    vec2 v = vec2(0.0);

    dice=vec3(hash3u(s + 0u)) / float(-1u) - vec3(0.5, 0.5, 0.0);
    v += dice.xy * smoothstep(1.0, 0.0, abs(p + dice.z));
    dice=vec3(hash3u(s + 1u)) / float(-1u) - vec3(0.5, 0.5, 1.0);
    v += dice.xy * smoothstep(1.0, 0.0, abs(p + dice.z));
    dice=vec3(hash3u(s + 2u)) / float(-1u) - vec3(0.5, 0.5, 2.0);
    v += dice.xy * smoothstep(1.0, 0.0, abs(p + dice.z));

    return 2.0 * v;
}

vec2 snare(float t, float q) {
    // envelope - exponential decay with initial hold
    float env = smoothstep(0.0, 0.001, t) * smoothstep(0.0, 0.001, q);
    env *= exp(-20.0 * max(t - 0.04, 0.0));

    // phase for body - 220Hz + fall
    float phase = 220.0 * t;
    phase += 4.0 * (1.0 - exp2(-t * 200.0));

    // oscillator - two sinewaves + noise
    vec2 osc = mix(
        mix(
            cis(TAU * phase), // 1x freq sine
            cis(1.5 * TAU * phase), // 1.5x freq sine
            0.4
        ),
        cheapnoise(128.0 * t) - cheapnoise(128.0 * t - 0.008), // noise
        0.3
    );

    return env * osc;
}

vec2 audio(float t, float f, float[4] options) {
    return snare(t, 1);
}