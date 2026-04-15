# Synth
![build](https://github.com/n-e-l/synth/actions/workflows/rust.yml/badge.svg)

A simple glsl audio editor. This was a personal playground so don't judge the code too hard.

- In-app code editor
- audio preview
- Open/save .glsl files
- shader compilation with error log

![preview](./preview.png)

## Building & running

You might need to have [Vulkan SDK](https://vulkan.lunarg.com) installed.  
Then build and run `synth`:
```
git clone https://github.com/n-e-l/synth.git
cd synth
cargo run --release
```