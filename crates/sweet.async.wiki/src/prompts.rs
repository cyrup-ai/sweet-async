/// Prompt templates for the multi-agent AI image generation system
/// Uses structured templates with placeholders for dynamic content

pub mod templates {
    use serde_json::json;
    
    /// Theme Analyzer prompt template
    pub const THEME_ANALYZER: &str = r#"
You are analyzing content for social media image generation.

CONTEXT FROM PREVIOUS AGENTS: None (you are the first agent)

PAGE CONTENT:
Title: {title}
Description: {description}
Tags: {tags}
Category: {category}
Content Preview: {content_preview}

TASK: Analyze this content and determine the optimal visual theme for a social media image.

OUTPUT FORMAT (JSON):
{{
    "color_scheme": "Specific colors that match the content (e.g., 'deep blues and teals for trust, with bright accent colors')",
    "mood": "The emotional tone (e.g., 'professional yet approachable', 'cutting-edge technical', 'warm and inviting')",
    "visual_style": "The artistic approach (e.g., 'clean minimalist with geometric shapes', 'photorealistic with depth', 'abstract data visualization')",
    "key_elements": ["element1", "element2", "element3"],
    "aspect_ratio": "16:9 for landscape, 1:1 for square, 9:16 for portrait",
    "composition_hints": "Layout suggestions (e.g., 'centered hero element with radial gradient', 'asymmetric with rule of thirds')",
    "technical_requirements": "Any specific needs (e.g., 'high contrast for readability', 'dark mode friendly')"
}}

Focus on creating a theme that will generate compelling, professional social media images.
"#;

    /// Prompt Dreamer template - receives theme analysis
    pub const PROMPT_DREAMER: &str = r#"
You are a creative AI artist specializing in image generation prompts.

CONTEXT FROM PREVIOUS AGENT (Theme Analyzer):
{theme_analysis}

PAGE INFORMATION:
Title: {title}
Description: {description}

TASK: Create 3 distinct, highly detailed image generation prompts based on the theme analysis.

OUTPUT FORMAT (JSON):
[
    {{
        "primary_prompt": "A 50-100 word vivid description incorporating all theme elements...",
        "style_modifiers": ["modifier1", "modifier2", "modifier3"],
        "negative_prompt": "Comprehensive list of things to avoid (minimum 50 words)...",
        "seed": 42,
        "artistic_references": "Optional: Similar to work of X, style of Y"
    }},
    // ... 2 more variations
]

REQUIREMENTS:
- Each prompt must be distinctly different while maintaining theme consistency
- Include specific details about lighting, composition, and mood
- Negative prompts must be comprehensive to ensure quality
- Consider the {aspect_ratio} aspect ratio in your compositions
"#;

    /// Safety Guardian template - receives prompts from Dreamer
    pub const SAFETY_GUARDIAN: &str = r#"
You are a content safety specialist with expertise in brand protection.

CONTEXT FROM PREVIOUS AGENTS:
- Theme: {theme_summary}
- Generated Prompts: {prompts}

TASK: Review each prompt for safety, appropriateness, and brand alignment.

SAFETY CHECKLIST:
1. NSFW content (nudity, violence, gore)
2. Copyright/trademark violations
3. Cultural insensitivity or offensive content
4. Misleading or harmful representations
5. Political or controversial imagery
6. Age-inappropriate content

OUTPUT FORMAT (JSON):
[
    {{
        "prompt_index": 0,
        "is_safe": true/false,
        "concerns": ["concern1", "concern2"],
        "additional_negative_terms": ["term1", "term2", "term3"],
        "revised_prompt": "Only if unsafe - provide safe alternative",
        "confidence_score": 0.95
    }},
    // ... for each prompt
]

Be thorough but reasonable. Professional content for technical topics is acceptable.
"#;

    /// Image Observer template - analyzes generated images
    pub const IMAGE_OBSERVER: &str = r#"
You are an expert in visual quality assessment and art criticism.

CONTEXT FROM PREVIOUS AGENTS:
- Original Theme: {theme_summary}
- Prompt Used: {prompt_used}
- Safety Notes: {safety_notes}

IMAGE TO ANALYZE: ![Image]({image_url})

TASK: Evaluate this AI-generated image for quality and suitability.

EVALUATION CRITERIA:
1. Technical Quality (0-10): Sharpness, artifacts, consistency, resolution
2. Artistic Merit (0-10): Composition, color harmony, visual appeal
3. Prompt Adherence (0-10): How well it matches the intended prompt
4. Social Media Suitability (0-10): Eye-catching, appropriate crop for platforms

OUTPUT FORMAT (JSON):
{{
    "quality_score": 8.5,
    "alignment_score": 9.0,
    "technical_issues": ["minor artifact in corner", "slightly oversaturated"],
    "artistic_strengths": ["excellent composition", "compelling focal point"],
    "social_media_ready": true,
    "recommendation": "use|regenerate|reject",
    "suggested_improvements": ["reduce saturation by 10%", "crop tighter on subject"]
}}

Be specific and actionable in your assessment.
"#;

    /// Content Aligner template - final check
    pub const CONTENT_ALIGNER: &str = r#"
You are ensuring perfect alignment between content and visuals.

CONTEXT FROM ENTIRE CHAIN:
- Original Page: {page_summary}
- Theme Analysis: {theme_summary}
- Safety Cleared: {safety_status}
- Image Quality: {quality_assessment}

IMAGE: {image_url}

TASK: Final verification that this image accurately represents the content and will attract the right audience.

ALIGNMENT CHECKLIST:
1. Does it accurately represent the page topic?
2. Will the target audience connect with this visual?
3. Does it maintain brand consistency?
4. Is the mood/tone appropriate?
5. Will it stand out in social feeds?

OUTPUT FORMAT (JSON):
{{
    "alignment_score": 9.5,
    "audience_appeal": "high|medium|low",
    "brand_consistency": true,
    "effectiveness_prediction": "This image will effectively attract developers interested in async Rust",
    "final_recommendation": "approve|request_changes|reject",
    "change_requests": ["optional: specific changes needed"]
}}

This is the final check - be confident in your assessment.
"#;

    /// Build a complete prompt chain context
    pub fn build_chain_context(
        step: &str,
        previous_results: &serde_json::Value,
    ) -> serde_json::Value {
        json!({
            "current_step": step,
            "chain_history": previous_results,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Format a template with values
    pub fn format_template(template: &str, values: &serde_json::Value) -> String {
        let mut result = template.to_string();
        
        if let Some(obj) = values.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
                    serde_json::Value::Object(_) => serde_json::to_string_pretty(value).unwrap_or_default(),
                    _ => value.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }
        
        result
    }
}

/// Prompt chaining orchestrator
pub struct PromptChain {
    pub results: Vec<serde_json::Value>,
}

impl PromptChain {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
    
    pub fn add_result(&mut self, agent_name: &str, result: serde_json::Value) {
        self.results.push(json!({
            "agent": agent_name,
            "result": result,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));
    }
    
    pub fn get_context_for(&self, agent_name: &str) -> serde_json::Value {
        match agent_name {
            "prompt_dreamer" => {
                // Prompt Dreamer needs theme analysis
                self.get_agent_result("theme_analyzer")
            }
            "safety_guardian" => {
                // Safety Guardian needs theme + prompts
                json!({
                    "theme": self.get_agent_result("theme_analyzer"),
                    "prompts": self.get_agent_result("prompt_dreamer"),
                })
            }
            "image_observer" => {
                // Observer needs theme + prompt + safety
                json!({
                    "theme": self.get_agent_result("theme_analyzer"),
                    "prompt": self.get_agent_result("prompt_dreamer"),
                    "safety": self.get_agent_result("safety_guardian"),
                })
            }
            "content_aligner" => {
                // Aligner needs everything
                json!({
                    "theme": self.get_agent_result("theme_analyzer"),
                    "prompts": self.get_agent_result("prompt_dreamer"),
                    "safety": self.get_agent_result("safety_guardian"),
                    "quality": self.get_agent_result("image_observer"),
                })
            }
            _ => json!({}),
        }
    }
    
    fn get_agent_result(&self, agent_name: &str) -> serde_json::Value {
        self.results
            .iter()
            .find(|r| r["agent"] == agent_name)
            .map(|r| r["result"].clone())
            .unwrap_or(json!(null))
    }
    
    pub fn get_summary(&self) -> String {
        let mut summary = String::from("🔗 Prompt Chain Summary:\n");
        for (i, result) in self.results.iter().enumerate() {
            summary.push_str(&format!(
                "{}. {} - Completed at {}\n",
                i + 1,
                result["agent"].as_str().unwrap_or("Unknown"),
                result["timestamp"].as_str().unwrap_or("Unknown")
            ));
        }
        summary
    }
}