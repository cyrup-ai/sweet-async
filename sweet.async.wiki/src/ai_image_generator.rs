use rig_core::{
    agent::{Agent, AgentBuilder},
    completion::{Chat, Completion, CompletionModel, Prompt, PromptError},
    providers::{anthropic, openai, mistral},
    pipeline::{self, agent_ops::extract, passthrough, Op},
    parallel,
    extractor::{Extractor, ExtractorBuilder},
};
use replicate_rust::{Replicate, Config as ReplicateConfig};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::collections::HashMap;
use std::env;

/// Multi-agent AI image generation system using Rig framework
/// Uses EXACTLY the models you specified:
/// - Theme Analyzer: Claude 4 Sonnet Latest (Anthropic)
/// - Prompt Dreamer: o4-mini (OpenAI)
/// - Safety Guardian: Mistral Large (Mistral)
/// - Image Generator: SDXL via Replicate API (using replicate-rust)
/// - Image Observer: Pixtral (Mistral's vision model)
/// - Content Aligner: o4-mini (OpenAI)
pub struct AiImageGenerator {
    theme_analyzer: Extractor<anthropic::CompletionModel, ThemeAnalysis>,
    prompt_dreamer: Agent<openai::CompletionModel>,
    safety_guardian: Agent<mistral::CompletionModel>,
    image_observer: Extractor<mistral::CompletionModel, QualityAssessment>,
    content_aligner: Extractor<openai::CompletionModel, QualityAssessment>,
    replicate: Replicate,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct PageContext {
    pub title: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThemeAnalysis {
    /// Color scheme that matches the content
    pub color_scheme: String,
    /// Emotional tone of the visual
    pub mood: String,
    /// Artistic approach
    pub visual_style: String,
    /// Key visual elements to include
    pub key_elements: Vec<String>,
    /// Recommended aspect ratio
    pub aspect_ratio: String,
    /// Layout and composition suggestions
    pub composition_hints: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ImagePrompts {
    /// Main description for image generation
    pub primary_prompt: String,
    /// Style modifiers to apply
    pub style_modifiers: Vec<String>,
    /// Things to avoid in the image
    pub negative_prompt: String,
    /// Seed for reproducibility
    pub seed: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SafetyReview {
    /// Whether the prompt is safe to use
    pub is_safe: bool,
    /// Any safety concerns found
    pub concerns: Vec<String>,
    /// Additional terms to add to negative prompt
    pub additional_negative_terms: Vec<String>,
    /// Revised prompt if original was unsafe
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct QualityAssessment {
    /// Overall quality score 0-10
    pub quality_score: f32,
    /// How well it aligns with content 0-10
    pub alignment_score: f32,
    /// Any quality issues found
    pub issues: Vec<String>,
    /// Recommendation: use, regenerate, or reject
    pub recommendation: String,
}

impl AiImageGenerator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize Anthropic client for Claude
        let anthropic_key = env::var("ANTHROPIC_API_KEY")?;
        let anthropic_client = anthropic::ClientBuilder::new(anthropic_key)
            .anthropic_version("2023-06-01")
            .build();
        
        // Initialize OpenAI client
        let openai_client = openai::Client::from_env();
        
        // Initialize Mistral client
        let mistral_key = env::var("MISTRAL_API_KEY")?;
        let mistral_client = mistral::Client::new(mistral_key);
        
        // Create specialized agents using EXTRACTORS for structured output
        
        // 1. Theme Analyzer: Claude 4 Sonnet Latest (structured extraction)
        let theme_analyzer = anthropic_client
            .extractor::<ThemeAnalysis>("claude-4-sonnet-latest")
            .preamble("You are a web design expert analyzing page content to determine appropriate visual themes for social media images.")
            .max_tokens(1024)
            .build();
        
        // 2. Prompt Dreamer: o4-mini (returns array, so use agent)
        let prompt_dreamer = openai_client
            .agent("o4-mini")
            .preamble("You are a creative AI artist specializing in writing detailed, evocative prompts for image generation. Create 3 distinct variations. Return a JSON array.")
            .build();
        
        // 3. Safety Guardian: Mistral Large (returns array, so use agent)
        let safety_guardian = mistral_client
            .agent(mistral::MISTRAL_LARGE)
            .preamble("You are a content safety specialist ensuring image prompts are appropriate, non-offensive, and brand-safe. Review each prompt and return a JSON array of safety reviews.")
            .build();
        
        // 4. Image Observer: Pixtral Latest (structured extraction)
        let image_observer = mistral_client
            .extractor::<QualityAssessment>("pixtral-latest")
            .preamble("You are an art critic and quality assessor for AI-generated images. Evaluate the technical and artistic quality of the provided image.")
            .build();
        
        // 5. Content Aligner: o4-mini (structured extraction)
        let content_aligner = openai_client
            .extractor::<QualityAssessment>("o4-mini")
            .preamble("You are a content alignment specialist ensuring images accurately represent page content and brand values.")
            .build();
        
        // 6. Initialize Replicate for image generation
        let replicate_key = env::var("REPLICATE_API_TOKEN")?;
        let replicate_config = ReplicateConfig { auth: replicate_key };
        let replicate = Replicate::new(replicate_config);
        
        Ok(Self {
            theme_analyzer,
            prompt_dreamer,
            safety_guardian,
            image_observer,
            content_aligner,
            replicate,
        })
    }
    
    /// Agent 1: Theme Analyzer - Reviews site layout and describes visual requirements
    pub async fn analyze_theme(&self, page: &PageContext) -> Result<ThemeAnalysis, Box<dyn std::error::Error>> {
        // Use structured extraction with detailed prompt template
        let prompt = format!(
            r#"CONTEXT: You are analyzing content for social media image generation.
            
            PAGE CONTENT:
            Title: {}
            Description: {}
            Tags: {:?}
            Category: {:?}
            Content Preview: {}
            
            TASK: Analyze this content and determine the optimal visual theme.
            
            REQUIREMENTS:
            - color_scheme: Specific colors that match the content (e.g., 'deep blues and teals for trust, with bright accent colors')
            - mood: The emotional tone (e.g., 'professional yet approachable', 'cutting-edge technical')
            - visual_style: The artistic approach (e.g., 'clean minimalist with geometric shapes', 'photorealistic with depth')
            - key_elements: List of specific visual elements to include
            - aspect_ratio: Choose from 16:9 (landscape), 1:1 (square), or 9:16 (portrait)
            - composition_hints: Layout suggestions (e.g., 'centered hero element with radial gradient')
            
            Focus on creating a theme that will generate compelling, professional social media images."#,
            page.title, page.description, page.tags, page.category,
            &page.content[..page.content.len().min(500)]
        );
        
        // Structured extraction automatically handles JSON parsing
        let analysis = self.theme_analyzer.extract(&prompt).await?;
        Ok(analysis)
    }
    
    /// Agent 2: The Dreamer - Creates multiple creative prompts
    pub async fn generate_prompts(&self, page: &PageContext, theme: &ThemeAnalysis) -> Result<Vec<ImagePrompts>, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"CONTEXT FROM PREVIOUS AGENT (Theme Analyzer):
            Color Scheme: {}
            Mood: {}
            Visual Style: {}
            Key Elements: {:?}
            Aspect Ratio: {}
            Composition: {}
            
            PAGE INFORMATION:
            Title: {}
            Description: {}
            
            TASK: Create 3 distinct, highly detailed image generation prompts based on the theme analysis.
            
            REQUIREMENTS FOR EACH PROMPT:
            - primary_prompt: A 50-100 word vivid description incorporating all theme elements
            - style_modifiers: List of specific style keywords (e.g., "octane render", "dramatic lighting", "bokeh")
            - negative_prompt: Comprehensive list of things to avoid (minimum 50 words)
            - seed: A number between 1-1000000 for reproducibility
            
            Make each prompt distinctly different while maintaining theme consistency.
            Consider the {} aspect ratio in your compositions.
            Return as JSON array."#,
            theme.color_scheme, theme.mood, theme.visual_style,
            theme.key_elements, theme.aspect_ratio, theme.composition_hints,
            page.title, page.description,
            theme.aspect_ratio
        );
        
        let response = self.prompt_dreamer.prompt(&prompt).await?;
        let prompts: Vec<ImagePrompts> = serde_json::from_str(&response)?;
        Ok(prompts)
    }
    
    /// Agent 3: The Guardian - Reviews prompts for safety and appropriateness
    pub async fn review_safety(&self, prompts: &[ImagePrompts]) -> Result<Vec<SafetyReview>, Box<dyn std::error::Error>> {
        let mut reviews = Vec::new();
        
        for img_prompt in prompts {
            let review_prompt = format!(
                r#"Review this image prompt for safety issues. Return JSON:
                
                Prompt: {}
                Negative: {}
                
                Check for: offensive content, NSFW, copyright issues, harmful representations
                
                Return JSON with:
                - is_safe: boolean
                - concerns: array of issues
                - additional_negative_terms: array of terms to add
                - revised_prompt: string (only if unsafe)
                "#,
                img_prompt.primary_prompt, img_prompt.negative_prompt
            );
            
            let response = self.safety_guardian.prompt(&review_prompt).await?;
            let review: SafetyReview = serde_json::from_str(&response)?;
            reviews.push(review);
        }
        
        Ok(reviews)
    }
    
    /// Agent 4: The Generator - Creates images using Replicate API
    pub async fn generate_images(&self, prompts: &[ImagePrompts]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut image_urls = Vec::new();
        
        // Using SDXL for high quality
        let model_version = "stability-ai/sdxl:39ed52f2a78e934b3ba6e2a89f5b1c712de7dfea535525255b1aa35c5565e08b";
        
        for prompt in prompts {
            let mut inputs = HashMap::new();
            inputs.insert("prompt", prompt.primary_prompt.clone());
            inputs.insert("negative_prompt", prompt.negative_prompt.clone());
            inputs.insert("width", "1024".to_string());
            inputs.insert("height", "1024".to_string());
            inputs.insert("num_outputs", "1".to_string());
            
            if let Some(seed) = prompt.seed {
                inputs.insert("seed", seed.to_string());
            }
            
            let result = self.replicate.run(model_version, inputs).await?;
            
            if let Some(output) = result.output.as_array() {
                if let Some(url) = output.first().and_then(|v| v.as_str()) {
                    image_urls.push(url.to_string());
                }
            }
        }
        
        Ok(image_urls)
    }
    
    /// Agent 5: The Observer - Analyzes generated images
    pub async fn analyze_images(&self, image_urls: &[String]) -> Result<Vec<QualityAssessment>, Box<dyn std::error::Error>> {
        let mut assessments = Vec::new();
        
        for url in image_urls {
            let assessment_prompt = format!(
                r#"IMAGE TO ANALYZE: ![Image]({})
                
                EVALUATION CRITERIA:
                1. Technical Quality (0-10): Sharpness, artifacts, consistency, resolution
                2. Artistic Merit (0-10): Composition, color harmony, visual appeal
                3. Social Media Suitability: Eye-catching, appropriate crop for platforms
                
                PROVIDE:
                - quality_score: Overall quality 0-10
                - alignment_score: Placeholder 8.0 (will be set by content aligner)
                - issues: List any technical problems (e.g., "minor artifact in corner", "slightly oversaturated")
                - recommendation: Choose "use", "regenerate", or "reject"
                
                Be specific and actionable in your assessment."#,
                url
            );
            
            // Structured extraction for quality assessment
            let assessment = self.image_observer.extract(&assessment_prompt).await?;
            assessments.push(assessment);
        }
        
        Ok(assessments)
    }
    
    /// Agent 6: The Aligner - Checks if images match page content
    pub async fn check_alignment(&self, page: &PageContext, image_urls: &[String], theme: &ThemeAnalysis) -> Result<Vec<QualityAssessment>, Box<dyn std::error::Error>> {
        let mut alignments = Vec::new();
        
        for url in image_urls {
            let alignment_prompt = format!(
                r#"CONTEXT FROM ENTIRE CHAIN:
                Original Page: "{}" - {}
                Theme Analysis: {} mood, {} style
                Target Audience: Technical professionals interested in {}
                
                IMAGE TO EVALUATE: {}
                
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
                
                This is the final check - be confident in your assessment."#,
                page.title, page.description,
                theme.mood, theme.visual_style,
                page.category.as_ref().unwrap_or(&"async Rust".to_string()),
                url
            );
            
            // Structured extraction for alignment assessment
            let alignment = self.content_aligner.extract(&alignment_prompt).await?;
            alignments.push(alignment);
        }
        
        Ok(alignments)
    }
    
    /// Orchestrate the entire multi-agent workflow using Rig's pipeline
    pub async fn generate_page_image(&self, page: PageContext) -> Result<String, Box<dyn std::error::Error>> {
        println!("🎨 Starting AI image generation pipeline for: {}", page.title);
        
        // Build the complete pipeline with proper chaining
        let pipeline = pipeline::new()
            // Stage 1: Theme Analysis (Claude 4 Sonnet)
            .map(|p: PageContext| {
                println!("👁️ Stage 1: Theme Analysis with Claude 4 Sonnet...");
                p
            })
            .chain(parallel!(
                passthrough(),
                |p: PageContext| async move {
                    self.analyze_theme(&p).await
                }
            ))
            
            // Stage 2: Creative Prompt Generation (o4-mini) - receives theme from Stage 1
            .map(|(page, theme)| {
                println!("💭 Stage 2: Prompt Generation with o4-mini...");
                (page, theme)
            })
            .chain(parallel!(
                passthrough(),
                |(p, t): (PageContext, Result<ThemeAnalysis, _>)| async move {
                    match t {
                        Ok(theme) => self.generate_prompts(&p, &theme).await,
                        Err(e) => Err(e),
                    }
                }
            ))
            
            // Stage 3: Safety Review (Mistral Large) - receives prompts from Stage 2
            .map(|((page, theme), prompts)| {
                println!("🛡️ Stage 3: Safety Review with Mistral Large...");
                (page, theme, prompts)
            })
            .chain(parallel!(
                passthrough(),
                |(_, _, prompts): (PageContext, Result<ThemeAnalysis, _>, Result<Vec<ImagePrompts>, _>)| async move {
                    match prompts {
                        Ok(p) => self.review_safety(&p).await,
                        Err(e) => Err(e),
                    }
                }
            ))
            
            // Apply safety revisions and generate images
            .map(|((page, theme, prompts), safety)| {
                println!("🎨 Stage 4: Image Generation with SDXL via Replicate...");
                
                // Apply safety revisions
                let mut safe_prompts = prompts.unwrap_or_default();
                if let Ok(reviews) = &safety {
                    for (i, review) in reviews.iter().enumerate() {
                        if i < safe_prompts.len() && !review.is_safe {
                            if let Some(revised) = &review.revised_prompt {
                                safe_prompts[i].primary_prompt = revised.clone();
                            }
                            safe_prompts[i].negative_prompt.push_str(&format!(
                                ", {}", 
                                review.additional_negative_terms.join(", ")
                            ));
                        }
                    }
                }
                
                (page, theme, safe_prompts)
            })
            
            // Generate images
            .chain(parallel!(
                passthrough(),
                |(_, _, prompts): (PageContext, Result<ThemeAnalysis, _>, Vec<ImagePrompts>)| async move {
                    self.generate_images(&prompts).await
                }
            ))
            
            // Stage 5: Quality Assessment (Pixtral) - analyzes generated images
            .map(|((page, theme, prompts), images)| {
                println!("🔍 Stage 5: Quality Assessment with Pixtral...");
                (page, theme, prompts, images)
            })
            .chain(parallel!(
                passthrough(),
                |(_, _, _, images): (PageContext, Result<ThemeAnalysis, _>, Vec<ImagePrompts>, Result<Vec<String>, _>)| async move {
                    match images {
                        Ok(urls) => self.analyze_images(&urls).await,
                        Err(e) => Err(e),
                    }
                }
            ))
            
            // Stage 6: Content Alignment (o4-mini) - final check
            .map(|((page, theme, prompts, images), quality)| {
                println!("✅ Stage 6: Content Alignment with o4-mini...");
                (page, theme, prompts, images, quality)
            })
            .map(|(page, theme, prompts, images, quality)| async move {
                let urls = images.unwrap_or_default();
                let theme_ref = theme.as_ref().ok();
                
                if urls.is_empty() || theme_ref.is_none() {
                    return Err("No images generated or theme analysis failed".into());
                }
                
                let alignments = self.check_alignment(&page, &urls, theme_ref.unwrap()).await?;
                
                // Select best image based on combined scores
                let best_index = alignments
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| {
                        let score_a = a.quality_score * 0.5 + a.alignment_score * 0.5;
                        let score_b = b.quality_score * 0.5 + b.alignment_score * 0.5;
                        score_a.partial_cmp(&score_b).unwrap()
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                
                println!("✨ Pipeline complete! Selected best image with scores: quality={}, alignment={}", 
                    alignments[best_index].quality_score,
                    alignments[best_index].alignment_score
                );
                
                Ok(urls[best_index].clone())
            });
        
        // Execute the pipeline
        pipeline.call(page).await
    }
}

/// Integration with the build system
pub async fn generate_ai_images_for_site(pages: Vec<PageContext>) -> Result<(), Box<dyn std::error::Error>> {
    let openai_key = std::env::var("OPENAI_API_KEY")?;
    let replicate_key = std::env::var("REPLICATE_API_TOKEN")?;
    
    let generator = AiImageGenerator::new(openai_key, replicate_key);
    
    for page in pages {
        match generator.generate_page_image(page.clone()).await {
            Ok(image_url) => {
                println!("✅ Generated image for {}: {}", page.title, image_url);
                // Download and save the image
                // Then trigger the social image generation pipeline
            }
            Err(e) => {
                println!("❌ Failed to generate image for {}: {}", page.title, e);
            }
        }
    }
    
    Ok(())
}