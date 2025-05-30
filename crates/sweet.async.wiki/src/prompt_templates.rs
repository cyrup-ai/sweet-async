/// Prompt templates using tera templating engine
use tera::{Tera, Context};
use once_cell::sync::Lazy;
use serde_json::json;

// Initialize Tera with our prompt templates
static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    let mut tera = Tera::default();
    
    // Theme Analyzer Template
    tera.add_raw_template("theme_analyzer", r#"
CONTEXT: You are analyzing content for social media image generation.

PAGE CONTENT:
Title: {{ title }}
Description: {{ description }}
Tags: {{ tags | json_encode }}
Category: {{ category | default(value="General") }}
Content Preview: {{ content_preview }}

TASK: Analyze this content and determine the optimal visual theme.

REQUIREMENTS:
- color_scheme: Specific colors that match the content (e.g., 'deep blues and teals for trust, with bright accent colors')
- mood: The emotional tone (e.g., 'professional yet approachable', 'cutting-edge technical')
- visual_style: The artistic approach (e.g., 'clean minimalist with geometric shapes', 'photorealistic with depth')
- key_elements: List of specific visual elements to include
- aspect_ratio: Choose from 16:9 (landscape), 1:1 (square), or 9:16 (portrait)
- composition_hints: Layout suggestions (e.g., 'centered hero element with radial gradient')

Focus on creating a theme that will generate compelling, professional social media images.
"#).unwrap();

    // Prompt Dreamer Template
    tera.add_raw_template("prompt_dreamer", r#"
CONTEXT FROM PREVIOUS AGENT (Theme Analyzer):
Color Scheme: {{ theme.color_scheme }}
Mood: {{ theme.mood }}
Visual Style: {{ theme.visual_style }}
Key Elements: {{ theme.key_elements | json_encode }}
Aspect Ratio: {{ theme.aspect_ratio }}
Composition: {{ theme.composition_hints }}

PAGE INFORMATION:
Title: {{ page.title }}
Description: {{ page.description }}

TASK: Create 3 distinct, highly detailed image generation prompts based on the theme analysis.

REQUIREMENTS FOR EACH PROMPT:
- primary_prompt: A 50-100 word vivid description incorporating all theme elements
- style_modifiers: List of specific style keywords (e.g., "octane render", "dramatic lighting", "bokeh")
- negative_prompt: Comprehensive list of things to avoid (minimum 50 words)
- seed: A number between 1-1000000 for reproducibility

Make each prompt distinctly different while maintaining theme consistency.
Consider the {{ theme.aspect_ratio }} aspect ratio in your compositions.
Return as JSON array.
"#).unwrap();

    // Safety Guardian Template
    tera.add_raw_template("safety_guardian", r#"
CONTEXT FROM PREVIOUS AGENTS:
- Theme: {{ theme_summary }}
- Generated Prompts: {{ prompts | json_encode }}

TASK: Review each prompt for safety, appropriateness, and brand alignment.

SAFETY CHECKLIST:
1. NSFW content (nudity, violence, gore)
2. Copyright/trademark violations
3. Cultural insensitivity or offensive content
4. Misleading or harmful representations
5. Political or controversial imagery
6. Age-inappropriate content

For each prompt, return:
- prompt_index: The index in the array
- is_safe: true/false
- concerns: List any issues found
- additional_negative_terms: Terms to add to negative prompt
- revised_prompt: Only if unsafe - provide safe alternative

Be thorough but reasonable. Professional content for technical topics is acceptable.
Return as JSON array.
"#).unwrap();

    // Image Observer Template
    tera.add_raw_template("image_observer", r#"
CONTEXT FROM PREVIOUS AGENTS:
- Original Theme: {{ theme_summary }}
- Prompt Used: {{ prompt_used }}
- Safety Notes: {{ safety_notes }}

IMAGE TO ANALYZE: ![Image]({{ image_url }})

EVALUATION CRITERIA:
1. Technical Quality (0-10): Sharpness, artifacts, consistency, resolution
2. Artistic Merit (0-10): Composition, color harmony, visual appeal
3. Social Media Suitability: Eye-catching, appropriate crop for platforms

PROVIDE:
- quality_score: Overall quality 0-10
- alignment_score: Placeholder 8.0 (will be set by content aligner)
- issues: List any technical problems
- recommendation: Choose "use", "regenerate", or "reject"

Be specific and actionable in your assessment.
"#).unwrap();

    // Content Aligner Template
    tera.add_raw_template("content_aligner", r#"
CONTEXT FROM ENTIRE CHAIN:
Original Page: "{{ page.title }}" - {{ page.description }}
Theme Analysis: {{ theme.mood }} mood, {{ theme.visual_style }} style
Target Audience: Technical professionals interested in {{ page.category | default(value="async Rust") }}

IMAGE TO EVALUATE: {{ image_url }}

ALIGNMENT CHECKLIST:
1. Does it accurately represent the page topic?
2. Will the target audience connect with this visual?
3. Does it maintain brand consistency?
4. Is the mood/tone appropriate?
5. Will it stand out in social feeds?

PROVIDE:
- quality_score: Placeholder 8.0 (already set by observer)
- alignment_score: How well it matches content 0-10
- issues: Any alignment problems
- recommendation: "use" if aligned, "regenerate" if not, "reject" if completely wrong

This is the final check - be confident in your assessment.
"#).unwrap();

    tera
});

/// Render a prompt template with context
pub fn render_prompt(template_name: &str, context: &Context) -> Result<String, tera::Error> {
    TEMPLATES.render(template_name, context)
}

/// Helper to create context for theme analyzer
pub fn theme_analyzer_context(page: &crate::ai_image_generator::PageContext) -> Context {
    let mut context = Context::new();
    context.insert("title", &page.title);
    context.insert("description", &page.description);
    context.insert("tags", &page.tags);
    context.insert("category", &page.category);
    context.insert("content_preview", &page.content.chars().take(500).collect::<String>());
    context
}

/// Helper to create context for prompt dreamer
pub fn prompt_dreamer_context(
    page: &crate::ai_image_generator::PageContext,
    theme: &crate::ai_image_generator::ThemeAnalysis,
) -> Context {
    let mut context = Context::new();
    context.insert("page", &json!({
        "title": page.title,
        "description": page.description
    }));
    context.insert("theme", &json!({
        "color_scheme": theme.color_scheme,
        "mood": theme.mood,
        "visual_style": theme.visual_style,
        "key_elements": theme.key_elements,
        "aspect_ratio": theme.aspect_ratio,
        "composition_hints": theme.composition_hints
    }));
    context
}

/// Helper to create context for safety guardian
pub fn safety_guardian_context(
    theme: &crate::ai_image_generator::ThemeAnalysis,
    prompts: &[crate::ai_image_generator::ImagePrompts],
) -> Context {
    let mut context = Context::new();
    context.insert("theme_summary", &format!("{} mood, {} style", theme.mood, theme.visual_style));
    context.insert("prompts", &prompts);
    context
}

/// Helper to create context for image observer
pub fn image_observer_context(
    theme: &crate::ai_image_generator::ThemeAnalysis,
    prompt: &crate::ai_image_generator::ImagePrompts,
    image_url: &str,
) -> Context {
    let mut context = Context::new();
    context.insert("theme_summary", &format!("{} mood, {} style", theme.mood, theme.visual_style));
    context.insert("prompt_used", &prompt.primary_prompt);
    context.insert("safety_notes", "All safety checks passed");
    context.insert("image_url", image_url);
    context
}

/// Helper to create context for content aligner
pub fn content_aligner_context(
    page: &crate::ai_image_generator::PageContext,
    theme: &crate::ai_image_generator::ThemeAnalysis,
    image_url: &str,
) -> Context {
    let mut context = Context::new();
    context.insert("page", &json!({
        "title": page.title,
        "description": page.description,
        "category": page.category
    }));
    context.insert("theme", &json!({
        "mood": theme.mood,
        "visual_style": theme.visual_style
    }));
    context.insert("image_url", image_url);
    context
}