use std::sync::LazyLock;

pub static AGENTS: &'static str = "0x424Fa11D84e5674809Fd0112eBa4f86d6C4ed2aD";
pub static ACCESS_CONTROLS: &'static str = "0x4F276081A4AC2d50eEE2aA6c78a3C4C06AAE9562";
pub static COLLECTION_MANAGER: &'static str = "0xBa53Fd19053fceFc91D091A02c71AbDcD79d856f";
pub static MARKET: &'static str = "0x6c7a9d566F6c2a9829B940b7571A220c70817c1a";
pub static REMIX_FEED: &'static str = "0x";
pub static ZERO_ADDRESS: &'static str = "0x0000000000000000000000000000000000000000";
pub static WGHO: &'static str = "0x6bDc36E20D267Ff0dd6097799f82e78907105e2F";
pub static BONSAI: &'static str = "0xB0588f9A9cADe7CD5f194a5fe77AcD6A58250f82";
pub static MONA: &'static str = "0x28547B5b6B405A1444A17694AC84aa2d6A03b3Bd";
pub static MODELS: &[&str] = &[
    "flux-dev-uncensored",
    "lustify-sdxl",
    "fluently-xl",
    "pony-realism",
];

pub static VENICE_API: &'static str = "https://api.venice.ai/api/v1/";
pub static LENS_API: &'static str = "https://api.lens.xyz/graphql";
pub static LENS_RPC_URL: &'static str = "https://rpc.lens.xyz";
pub static INFURA_GATEWAY: &'static str = "https://thedial.infura-ipfs.io/";
pub static LENS_CHAIN_ID: LazyLock<u64> = LazyLock::new(|| 232);
pub static ARTISTS: &[&str] = &[
    "0xae2d4A8191B55E9feA86934dc4FbC89eEE22efB6",
    "0x8860B76fEBC66092809B490A96E65BAD71c3Ac65",
    "0x1a13EE92680Cc847e27a6bF66303491d2b9AEcE7",
    "0xe3e76B32a1F66996d3Cb64D5599E5e6387D8C883",
    "0x03F034B0dF65887EAEEAe851fa668E72cC708581",
    "0xe5E949FBEdD829beD5e9283da4d50325D8F0F5a6",
    "0xCb30574340d013F8A8aeC29f828a12b7D53641bD",
    "0x9C9F99589111d181a7C58AfA6a53E469a187F663",
    "0x26E3F8d2065a9BFDDdfFBA7fddEA0d7eb0eCFF6f",
    "0x2dc0992cE7078b105eed1DFfce80db712eDA9792",
];
pub static STYLE_PRESETS: &[&str] = &[
    "Analog Film",
    "Line Art",
    "Neon Punk",
    "Pixel Art",
    "Texture",
    "Abstract",
    "Graffiti",
    "Pointillism",
    "Pop Art",
    "Psychedelic",
    "Renaissance",
    "Surrealist",
    "Retro Arcade",
    "Retro Game",
    "Street Fighter",
    "Legend of Zelda",
    "Gothic",
    "Grunge",
    "Horror",
    "Minimalist",
    "Monochrome",
    "Nautical",
    "Collage",
    "Kirigami",
    "Film Noir",
    "HDR",
    "Long Exposure",
    "Neon Noir",
    "Silhouette",
    "Tilt-Shift",
];
pub static SAMPLE_PROMPT:&'static str = "A hyper-detailed, painterly portrait of an anthropomorphic white cat standing upright, with soft fur rendered in fine, realistic brushstrokes. Its luminous yellow-green eyes are large and expressive, reflecting ambient light with subtle catch highlights. The cat wears an elaborate, mid-length cloak with finely embroidered floral patterns—wildflowers, vines, and gold-thread filigree—that flow naturally around the fabric folds. The fabric texture is tactile, slightly weathered linen layered over silk, with subtle fringe and hand-sewn imperfections. Rich sky-blue and ochre accents line the collar and edges, knotted at the neck with a small ornate clasp. The cat gently holds a sleek, matte-black handheld video game console—contrasting yet harmonizing with the surrounding natural motif. The device glows faintly, its screen casting a cool modern light across the paws. The cat is seen from a low angle, looking down at the device with a curious and slightly mischievous expression, as if it has just discovered a hidden level. The background is a deep velvet blue, softly gradiented with painterly clouding and blurred wildflower stalks rising into shadow. The lighting is diffuse and natural, like early evening after rain—subtle volumetric softness, no hard shadows. The scene is framed like a formal oil portrait, with a shallow depth of field and atmospheric occlusion around the edges. The style is reminiscent of Studio Ghibli meets classical European storybook illustration, with a touch of surreal whimsy. The overall effect is enchanting, gentle, and slightly uncanny—a quiet tension between timeless forest nobility and portable technology. The scene is set in a serene, mystical forest clearing, with the faint sound of a distant waterfall and the soft rustling of leaves, evoking a sense of tranquility and wonder.";
pub static NEGATIVE_PROMPT:&'static str = "(worst quality, low quality), (bad face), (deformed eyes), (bad eyes), ((extra hands)), extra fingers, too many fingers, fused fingers, bad arm, distorted arm, extra arms, fused arms, extra legs, missing leg, disembodied leg, extra nipples, detached arm, liquid hand, inverted hand, disembodied limb, oversized head, extra body, extra navel, (hair between eyes), twins, doubles";
pub static NEGATIVE_PROMPT_IMAGE:&'static str = "terrible quality, text, logo, signature,  amateur, b&w, duplicate, mutilated, extra fingers, mutated hands, deformed, cloned face, bad anatomy,  malformed limbs, missing arms, missing legs, extra arms, extra legs, mutated hands, fused fingers, too many fingers, tripod, tube, tiling, extra limbs, extra legs, cross-eye, out of frame";
pub static INPUT_IRL_FASHION: &[&str] = &[
    "QmUMwVnHKx73RcSMoVFcKQGb3aeErWvb67i9mA2sX2jehk",
    "QmZJwkav1ELzpiedvQqjex7VsBH1Y4ops5UEadeQXnHXAB",
    "QmSpbXasjgYGjWTkSxAZmyqj4Ht8x43jdsv9H6AHNJZ9Vy",
    "QmXu3fbBesEGjDqp9qaQ3Z31amkzyky51vXwtrzi4NPTUu",
    "QmaEVf4G1DosANrgMk2TV9uWKuW4fxfYhoFQpGDcCMU8TY",
    "QmWiG1U7GnLQxet2v75e6W4MucRU2F2g3ZBdVY2NZnSAUk",
    "QmTqngJsrp4X1npb6q1FTyvPAeG3Hskym69EsirAVDPiao",
    "QmVZDZMF173c7CmoxHT2AWk5fWpSJv8JCPGV58TP3Av66d",
    "QmVtR7TuoXVYfNFfLhGwkkV6G7bLmu3Wyg9k2PF7n8a8Td",
    "QmZKcmScm6y7CNgh2y1KGdyR4JsME9vqd4eGetHR8QunRD",
    "QmTa17X6X8T5AjM3NJW6DeAD6z3FK5nqfWste3uVMJvWGr",
    "QmcTMfyieV3eJrCqMHyxVwmugQ8nC8Vjb4xwdTRH13zzMy",
    "QmTsZkzYvyMpPXc2kqtUwdaPHZfsjrcW6yUGrNMDHs87B9",
    "QmY3r6wxmXaJCYTCqguiWZftJfSb8hp2V19B3LAyodVKZ9",
    "QmQaPMCwu7fXojLLL7iDmtCmhCKJqPTp946g8ZQN47s939",
    "QmPrXRvb3nZDHt3c8AF8LSuFQktbXYGGcjpFX7kGfaJ77a",
];
