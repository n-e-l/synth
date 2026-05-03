use std::collections::BTreeSet;
use egui_code_editor::Syntax;

pub fn slang_syntax() -> Syntax {
    let keywords: BTreeSet<&'static str> = [
        // Control flow
        "if", "else", "for", "while", "do", "switch", "case", "default",
        "break", "continue", "return", "discard",
        "try", "throw", "throws",
        // Declaration / type system
        "struct", "class", "enum", "interface", "extension",
        "typedef", "typealias", "namespace", "using",
        "import", "module", "implementing",
        "func", "var", "let", "property",
        "get", "set", "init", "__init", "subscript", "This",
        // Modifiers
        "in", "out", "inout", "ref",
        "const", "static", "extern", "export",
        "public", "private", "internal",
        "inline", "noinline", "__intrinsic",
        "uniform", "varying",
        "groupshared", "globallycoherent",
        "row_major", "column_major",
        "linear", "centroid", "nointerpolation", "noperspective", "sample",
        "precise", "abstract", "override", "virtual", "final",
        "mutating", "stage", "__exported",
        // Literals
        "true", "false", "nullptr", "NULL",
        // Preprocessor
        "#define", "#undef", "#if", "#ifdef", "#ifndef",
        "#else", "#elif", "#endif", "#include", "#pragma", "#line",
    ].into();

    let types: BTreeSet<&'static str> = [
        // Scalars
        "void", "bool",
        "int", "int8_t", "int16_t", "int32_t", "int64_t",
        "uint", "uint8_t", "uint16_t", "uint32_t", "uint64_t",
        "half", "float", "double",
        "float16_t", "float32_t", "float64_t",
        // Vectors
        "bool1", "bool2", "bool3", "bool4",
        "int1", "int2", "int3", "int4",
        "uint1", "uint2", "uint3", "uint4",
        "half1", "half2", "half3", "half4",
        "float1", "float2", "float3", "float4",
        "double1", "double2", "double3", "double4",
        "vector",
        // Matrices
        "float1x1", "float1x2", "float1x3", "float1x4",
        "float2x1", "float2x2", "float2x3", "float2x4",
        "float3x1", "float3x2", "float3x3", "float3x4",
        "float4x1", "float4x2", "float4x3", "float4x4",
        "double2x2", "double3x3", "double4x4",
        "int2x2", "int3x3", "int4x4",
        "matrix",
        // Resources
        "Texture1D", "Texture2D", "Texture3D", "TextureCube",
        "Texture1DArray", "Texture2DArray", "TextureCubeArray",
        "Texture2DMS", "Texture2DMSArray",
        "RWTexture1D", "RWTexture2D", "RWTexture3D",
        "RWTexture1DArray", "RWTexture2DArray",
        "Buffer", "RWBuffer", "ByteAddressBuffer", "RWByteAddressBuffer",
        "StructuredBuffer", "RWStructuredBuffer", "AppendStructuredBuffer",
        "ConsumeStructuredBuffer",
        "ConstantBuffer", "ParameterBlock",
        "SamplerState", "SamplerComparisonState",
        "RaytracingAccelerationStructure",
        "SubpassInput", "SubpassInputMS",
    ].into();

    Syntax::new("slang")
        .with_case_sensitive(true)
        .with_comment("//")
        .with_comment_multiline(["/*", "*/"])
        .with_keywords(keywords)
        .with_types(types)
}

pub fn glsl_syntax() -> Syntax {
    let keywords: BTreeSet<&'static str> = [
        // Control flow
        "if", "else", "for", "while", "do", "switch", "case", "default",
        "break", "continue", "return", "discard",
        // Declaration
        "struct", "void",
        // Qualifiers - storage
        "const", "uniform", "buffer", "shared", "attribute", "varying",
        "in", "out", "inout",
        "centroid", "patch", "sample",
        "flat", "smooth", "noperspective",
        "coherent", "volatile", "restrict", "readonly", "writeonly",
        // Qualifiers - precision
        "highp", "mediump", "lowp", "precision",
        // Qualifiers - layout
        "layout",
        // Qualifiers - other
        "invariant", "precise",
        // Subroutine
        "subroutine",
    ].into();

    let types: BTreeSet<&'static str> = [
        // Scalars
        "bool", "int", "uint", "float", "double",
        // Float vectors
        "vec2", "vec3", "vec4",
        // Double vectors
        "dvec2", "dvec3", "dvec4",
        // Int vectors
        "ivec2", "ivec3", "ivec4",
        // Uint vectors
        "uvec2", "uvec3", "uvec4",
        // Bool vectors
        "bvec2", "bvec3", "bvec4",
        // Float matrices
        "mat2", "mat3", "mat4",
        "mat2x2", "mat2x3", "mat2x4",
        "mat3x2", "mat3x3", "mat3x4",
        "mat4x2", "mat4x3", "mat4x4",
        // Double matrices
        "dmat2", "dmat3", "dmat4",
        "dmat2x2", "dmat2x3", "dmat2x4",
        "dmat3x2", "dmat3x3", "dmat3x4",
        "dmat4x2", "dmat4x3", "dmat4x4",
        // Floating point samplers
        "sampler1D", "sampler2D", "sampler3D", "samplerCube",
        "sampler1DShadow", "sampler2DShadow", "samplerCubeShadow",
        "sampler1DArray", "sampler2DArray",
        "sampler1DArrayShadow", "sampler2DArrayShadow",
        "samplerCubeArray", "samplerCubeArrayShadow",
        "sampler2DMS", "sampler2DMSArray",
        "samplerBuffer",
        "sampler2DRect", "sampler2DRectShadow",
        // Int samplers
        "isampler1D", "isampler2D", "isampler3D", "isamplerCube",
        "isampler1DArray", "isampler2DArray",
        "isamplerCubeArray",
        "isampler2DMS", "isampler2DMSArray",
        "isamplerBuffer", "isampler2DRect",
        // Uint samplers
        "usampler1D", "usampler2D", "usampler3D", "usamplerCube",
        "usampler1DArray", "usampler2DArray",
        "usamplerCubeArray",
        "usampler2DMS", "usampler2DMSArray",
        "usamplerBuffer", "usampler2DRect",
        // Images
        "image1D", "image2D", "image3D", "imageCube",
        "image1DArray", "image2DArray", "imageCubeArray",
        "image2DMS", "image2DMSArray",
        "imageBuffer", "image2DRect",
        "iimage1D", "iimage2D", "iimage3D", "iimageCube",
        "iimage1DArray", "iimage2DArray", "iimageCubeArray",
        "iimage2DMS", "iimage2DMSArray",
        "iimageBuffer", "iimage2DRect",
        "uimage1D", "uimage2D", "uimage3D", "uimageCube",
        "uimage1DArray", "uimage2DArray", "uimageCubeArray",
        "uimage2DMS", "uimage2DMSArray",
        "uimageBuffer", "uimage2DRect",
        // Atomic
        "atomic_uint",
    ].into();

    let special: BTreeSet<&'static str> = [
        // Boolean literals
        "true", "false",
        // Built-in variables
        "gl_Position", "gl_PointSize", "gl_ClipDistance", "gl_CullDistance",
        "gl_FragCoord", "gl_FrontFacing", "gl_FragDepth", "gl_PointCoord",
        "gl_SampleID", "gl_SamplePosition", "gl_SampleMask",
        "gl_VertexID", "gl_InstanceID", "gl_VertexIndex", "gl_InstanceIndex",
        "gl_PrimitiveID", "gl_InvocationID", "gl_Layer", "gl_ViewportIndex",
        "gl_TessLevelOuter", "gl_TessLevelInner", "gl_TessCoord", "gl_PatchVerticesIn",
        "gl_GlobalInvocationID", "gl_LocalInvocationID", "gl_LocalInvocationIndex",
        "gl_WorkGroupID", "gl_WorkGroupSize", "gl_NumWorkGroups",
        "gl_SubgroupSize", "gl_SubgroupInvocationID",
    ].into();

    Syntax::new("glsl")
        .with_case_sensitive(true)
        .with_comment("//")
        .with_comment_multiline(["/*", "*/"])
        .with_keywords(keywords)
        .with_types(types)
        .with_special(special)
}