
// From https://github.com/0b5vr/wavenerd-dubplates/blob/main/shaders/20250712_drum_machine_tutorial.glsl
// All copyright goes to them

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

// Method called by the main shader
vec2 audio(float t, float f, float[4] options) {
    return 0.5 * kick(t, f);
}