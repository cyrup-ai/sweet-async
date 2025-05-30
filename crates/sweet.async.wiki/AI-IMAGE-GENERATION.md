# AI-Powered Social Image Generation

This documentation explains the multi-agent AI system that automatically generates contextually relevant, high-quality social media images for pages without explicit `og_image` specifications.

## Overview

The system uses **Rig** (Rust LLM orchestration framework) to coordinate multiple AI agents with the EXACT latest models:

1. **Theme Analyzer** - Claude 4 Sonnet Latest (Anthropic) for deep content analysis
2. **The Dreamer** - o4-mini (OpenAI) for efficient creative prompts
3. **The Guardian** - Mistral Large (Mistral) for safety review
4. **The Generator** - SDXL via Replicate API for image generation
5. **The Observer** - Pixtral Latest (Mistral's latest vision model) for visual quality assessment
6. **The Aligner** - o4-mini (OpenAI) for content alignment

Each agent uses the specific model chosen for its unique strengths and capabilities.

## Setup

### 1. Install Dependencies

The system uses these Rust crates:
- `rig-core = "0.5.0"` - LLM agent orchestration framework
- `replicate-rust = "0.1"` - Unofficial Replicate API client
- `tokio` - Async runtime
- `serde` - JSON serialization

Add to your `Cargo.toml`:
```toml
[dependencies]
rig-core = "0.5.0"
replicate-rust = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.11", features = ["json"] }
image = "0.25"
```

### 2. Set Environment Variables

```bash
export ANTHROPIC_API_KEY="your-anthropic-api-key"  # For Claude 4 Sonnet
export OPENAI_API_KEY="your-openai-api-key"       # For o4-mini
export MISTRAL_API_KEY="your-mistral-api-key"     # For Mistral Large & Pixtral Latest
export REPLICATE_API_TOKEN="your-replicate-token"  # For SDXL image generation
```

### 3. Optional: Unsplash Fallback

```bash
export UNSPLASH_ACCESS_KEY="your-unsplash-key"
```

## How It Works

### Build-Time Queue System

1. During `cargo build`, pages without `og_image` are detected
2. Page context is saved to `static/assets/img/ai-generated/{slug}.context.json`
3. A marker file `{slug}.generating` prevents duplicate generation

### AI Generation Process

Run the AI generator:

```bash
cargo run --bin generate-ai-images
```

## Rig Pipeline Architecture

The system uses Rig's pipeline and chaining features for proper prompt flow:

```rust
let pipeline = pipeline::new()
    // Stage 1: Theme Analysis
    .chain(parallel!(
        passthrough(),  // Pass page context forward
        |page| async { analyze_theme(&page).await }
    ))
    
    // Stage 2: Prompt Generation (receives theme from Stage 1)
    .chain(parallel!(
        passthrough(),  // Pass (page, theme) forward
        |(page, theme)| async { generate_prompts(&page, &theme).await }
    ))
    
    // Stage 3: Safety Review (receives prompts from Stage 2)
    .chain(parallel!(
        passthrough(),
        |(_, _, prompts)| async { review_safety(&prompts).await }
    ))
    
    // Continue chaining through all agents...
```

### Pipeline Flow

```
┌─────────────────┐
│  Page Content   │
└────────┬────────┘
         │
    ┌────▼────┐
    │ Stage 1 │ Theme Analysis (Claude 4 Sonnet)
    │ parallel│ → (page, theme)
    └────┬────┘
         │
    ┌────▼────┐
    │ Stage 2 │ Prompt Generation (o4-mini)
    │ parallel│ → (page, theme, prompts)
    └────┬────┘
         │
    ┌────▼────┐
    │ Stage 3 │ Safety Review (Mistral Large)
    │ parallel│ → (page, theme, safe_prompts)
    └────┬────┘
         │
    ┌────▼────┐
    │ Stage 4 │ Image Generation (SDXL)
    │ async   │ → image_urls
    └────┬────┘
         │
    ┌────▼────┐
    │ Stage 5 │ Quality Assessment (Pixtral)
    │ parallel│ → quality_scores
    └────┬────┘
         │
    ┌────▼────┐
    │ Stage 6 │ Content Alignment (o4-mini)
    │ async   │ → final_selection
    └────┬────┘
         │
    ┌────▼────┐
    │  Best   │
    │  Image  │
    └─────────┘
```

### Key Features

1. **Parallel Processing**: Uses `parallel!` macro to run independent operations concurrently
2. **Context Passing**: Each stage receives outputs from previous stages
3. **Error Handling**: Pipeline propagates errors through the chain
4. **Type Safety**: Rig's pipeline ensures type-safe data flow between stages

### Integration with Social Image Pipeline

Once AI images are generated:
1. Run `cargo build` again
2. The build system detects AI-generated images
3. Creates all 50+ social media sizes automatically

## Agent Details

### Creating Agents with Rig

Each agent uses the EXACT model you specified with proper preambles:

```rust
// 1. Claude 4 Sonnet Latest for theme analysis
let theme_analyzer = anthropic_client
    .agent("claude-4-sonnet-latest")
    .preamble("You are a web design expert analyzing page content 
              to determine appropriate visual themes for social 
              media images. Always respond with valid JSON.")
    .max_tokens(1024)  // Required for Anthropic
    .build();

// 2. o4-mini for prompt generation
let prompt_dreamer = openai_client
    .agent("o4-mini")
    .preamble("You are a creative AI artist specializing in 
              writing detailed, evocative prompts for image 
              generation. Create vivid, specific descriptions. 
              Always respond with valid JSON.")
    .build();

// 3. Mistral Large for safety
let safety_guardian = mistral_client
    .agent(mistral::MISTRAL_LARGE)
    .preamble("You are a content safety specialist ensuring 
              image prompts are appropriate, non-offensive, 
              and brand-safe. Be thorough but reasonable. 
              Always respond with valid JSON.")
    .build();

// 4. Pixtral Latest for image observation
let image_observer = mistral_client
    .agent("pixtral-latest")
    .preamble("You are an art critic and quality assessor for 
              AI-generated images. Evaluate technical and 
              artistic quality. Always respond with valid JSON.")
    .build();

// 5. o4-mini for content alignment
let content_aligner = openai_client
    .agent("o4-mini")
    .preamble("You are a content alignment specialist ensuring 
              images accurately represent page content and 
              brand values. Always respond with valid JSON.")
    .build();
```

### Prompt Templates

Each agent receives context from previous stages through the pipeline:

```rust
// Theme Analyzer receives page context
let theme_prompt = format!(
    "Analyze this page and provide visual theme as JSON:\n\n\
     Title: {}\n\
     Description: {}\n\
     Tags: {:?}\n\
     Content: {}\n\n\
     Return JSON with: color_scheme, mood, visual_style, \
     key_elements, aspect_ratio, composition_hints",
    page.title, page.description, page.tags, &page.content[..500]
);

// Prompt Dreamer receives theme analysis
let prompt_template = format!(
    "Based on this theme analysis, create 3 image prompts:\n\n\
     Theme: {:?}\n\
     Page: {}\n\n\
     Return JSON array with: primary_prompt, style_modifiers, \
     negative_prompt, seed",
    theme, page.title
);

// Each subsequent agent builds on previous context
```

### Agent 1: Theme Analyzer (Claude 4 Sonnet Latest)

Uses Anthropic's Claude 4 Sonnet Latest (`claude-4-sonnet-latest`):
- **Model**: The latest Claude 4 Sonnet for superior understanding
- **Color scheme**: Appropriate colors based on content
- **Mood**: Professional, technical, playful, etc.
- **Visual style**: Photorealistic, illustration, abstract
- **Key elements**: What should appear in the image
- **Composition**: Layout and framing suggestions

Responds with structured JSON for easy parsing.

### Agent 2: The Dreamer (o4-mini)

Uses OpenAI's o4-mini for cost-effective creativity:
- **Model**: `o4-mini` - OpenAI's latest mini model
- **Primary prompt**: 50-100 word vivid descriptions
- **Style modifiers**: Lighting, camera angle, artistic style
- **Negative prompt**: Comprehensive list of elements to avoid
- **Seed**: For reproducibility

### Agent 3: The Guardian (Mistral Large)

Uses Mistral Large for thorough safety analysis:
- **Model**: `mistral::MISTRAL_LARGE` - Mistral's most capable model
- NSFW or inappropriate content detection
- Copyright/trademark issues
- Cultural sensitivity
- Misleading representations

Adds defensive negative keywords and can revise prompts entirely.

### Agent 4: The Generator

Uses Replicate's SDXL model via the replicate-rust crate:
- Model: `stability-ai/sdxl:39ed52f2a78e934b3ba6e2a89f5b1c712de7dfea535525255b1aa35c5565e08b`
- High resolution: 1024x1024 base
- Supports custom seeds for reproducibility
- Handles negative prompts for better control

```rust
let mut inputs = HashMap::new();
inputs.insert("prompt", prompt.primary_prompt.clone());
inputs.insert("negative_prompt", prompt.negative_prompt.clone());
inputs.insert("width", "1024".to_string());
inputs.insert("height", "1024".to_string());

let result = self.replicate.run(model_version, inputs).await?;
```

### Agent 5: The Observer (Pixtral Latest)

Uses Mistral's latest Pixtral vision model for image quality assessment:
- **Model**: `pixtral-latest` - Mistral's latest multimodal vision model
- Analyzes images directly from URLs
- Technical quality (sharpness, artifacts)
- Artistic merit (composition, appeal)
- Social media suitability
- Overall quality score (0-10)

Image prompts use markdown format:
```rust
let prompt = format!("Analyze this image:\n\n![Image]({})\n\n...", url);
```

### Agent 6: The Aligner (o4-mini)

Uses o4-mini for efficient content alignment:
- **Model**: `o4-mini` - Fast and effective for comparison tasks
- Image represents content accurately
- Appropriate for the topic
- Matches intended mood/style
- Appeals to target audience

## Fallback Strategy

The system has multiple fallback levels:

1. **AI Generation** (if API keys available)
2. **Unsplash Semantic Search** (if Unsplash key available)
3. **Local Banner Pool** (always available)
4. **Placeholder Image** (last resort)

## Example Page Configuration

### Minimal (AI will analyze and generate)
```toml
+++
title = "Building Microservices"
description = "Learn microservice architecture patterns"
+++
```

### With Hints (Better AI results)
```toml
+++
title = "Kubernetes Orchestration"
description = "Deploy and manage containerized applications"
tags = ["cloud", "containers", "devops", "kubernetes"]
keywords = "kubernetes, k8s, container orchestration, cloud native"
category = "infrastructure"
+++
```

### Explicit Image (Bypasses AI)
```toml
+++
title = "My Specific Design"
og_image = "/assets/img/my-custom-image.jpg"
+++
```

## Quality Control

The system ensures quality through:

1. **Multiple prompts**: 3-4 variations per page
2. **Safety filtering**: Inappropriate content blocked
3. **Quality thresholds**: Images below 7/10 rejected
4. **Alignment scoring**: Must match content
5. **Regeneration**: Automatic retry if all fail

## Cost Optimization

- **Caching**: AI images cached locally
- **Reuse**: Same slug always gets same image
- **Selective**: Only generates for pages without images
- **Efficient**: Batch processing available

## Monitoring

Build output shows progress:
```
🎨 Starting AI image generation for: Building Microservices
👁️ Agent 1: Analyzing page theme...
💭 Agent 2: Dreaming up image prompts...
🛡️ Agent 3: Reviewing prompts for safety...
🎨 Agent 4: Generating images with Replicate...
🔍 Agent 5: Assessing image quality...
✅ Agent 6: Checking content alignment...
✨ Selected best image with scores: quality=8.5, alignment=9.0
```

## Attribution

Generated images include attribution files:
```
static/assets/img/ai-generated/
├── page-slug.jpg
├── page-slug.attribution.txt
└── page-slug.context.json
```

## Advanced Features

### Custom Model Selection

Modify `model_version` in `generate_images()` to use different models:
- SDXL for general use
- Stable Diffusion 2.1 for faster generation
- Custom fine-tuned models for specific styles

### Prompt Engineering

The system uses advanced prompt techniques:
- Detailed scene descriptions
- Specific style instructions
- Comprehensive negative prompts
- Artistic references

### Batch Processing

Process multiple pages efficiently:
```bash
# Queue all pages
cargo build

# Generate all AI images
cargo run --bin generate-ai-images

# Generate social sizes
cargo build
```

## Troubleshooting

### No images generated?
- Check API keys are set correctly
- Verify network connectivity
- Check build output for errors

### Poor quality images?
- Add more descriptive tags/keywords
- Provide richer page content
- Adjust quality thresholds

### Wrong style?
- Use category field to hint at style
- Add style-related tags
- Provide more context in description