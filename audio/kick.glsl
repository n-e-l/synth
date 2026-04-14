
// From https://github.com/0b5vr/wavenerd-dubplates/blob/main/shaders/20250712_drum_machine_tutorial.glsl
// All copyright goes to them

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

// == drums ========================================================================================
vec2 kick(float t, float q) {
    // envelope
    float env = smoothstep(0.0, 0.001, q);
    env *= smoothstep(0.3, 0.1, t);

    // phase - 50Hz + fall
    float phase = 50.0 * t;
    phase += 8.0 * (1.0 - exp2(-50.0 * t));

    // oscillator - simple sinewave
    vec2 osc = vec2(sin(TAU * phase));

    // transient
    float tphase = 5.0 * (1.0 - exp2(-400.0 * t));
    osc += sin(TAU * tphase);

    // add overdrive to osc
    osc = tanh(osc);

    return env * osc;
}

vec2 audio(float t, float f, float[4] options) {
    return 0.5 * kick(t, f);
}