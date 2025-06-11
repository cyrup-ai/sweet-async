use image::{DynamicImage, ImageFormat, imageops::FilterType};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub mod ai_image_generator;
pub mod prompt_templates;

/// Social media image specifications
/// Each entry: (name, logical_width, logical_height, densities)
/// Densities: (suffix, scale_factor)
const SOCIAL_SPECS: &[(&str, u32, u32, &[(&str, f32)])] = &[
    // Open Graph universal (1.91:1)
    ("og", 1200, 630, &[("-1x", 1.0), ("", 2.0), ("-3x", 3.0)]),
    
    // Square format
    ("og-square", 1200, 1200, &[("-1x", 1.0), ("", 2.0)]),
    
    // Twitter optimized (16:9)
    ("twitter", 1200, 675, &[("-1x", 1.0), ("", 2.0)]),
    
    // LinkedIn specific
    ("linkedin", 1200, 627, &[("-1x", 1.0), ("", 2.0)]),
    
    // Mobile portrait
    ("mobile", 1080, 1920, &[("-1x", 1.0), ("", 2.0), ("-3x", 3.0)]),
    
    // Tablet landscape
    ("tablet", 1920, 1200, &[("-1x", 1.0), ("", 2.0)]),
    
    // WhatsApp
    ("whatsapp", 1200, 630, &[("-1x", 1.0), ("", 2.0)]),
    
    // Instagram Story
    ("instagram-story", 1080, 1920, &[("-1x", 1.0), ("", 2.0)]),
    
    // Pinterest
    ("pinterest", 1000, 1500, &[("-1x", 1.0), ("", 2.0)]),
    
    // Discord
    ("discord", 1200, 630, &[("-1x", 1.0), ("", 2.0)]),
];

/// Android density buckets with their scale factors
const ANDROID_DENSITIES: &[(&str, f32)] = &[
    ("ldpi", 0.75),
    ("mdpi", 1.0),
    ("hdpi", 1.5),
    ("xhdpi", 2.0),
    ("xxhdpi", 3.0),
    ("xxxhdpi", 4.0),
];

pub struct SocialImageGenerator {
    source_dir: PathBuf,
    output_dir: PathBuf,
}

impl SocialImageGenerator {
    pub fn new(source_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self { source_dir, output_dir }
    }
    
    /// Process a page's og_image and generate all required sizes
    pub fn generate_for_page(&self, page_slug: &str, og_image_path: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let source_path = self.source_dir.join(og_image_path);
        let img = image::open(&source_path)?;
        
        let mut generated_images = HashMap::new();
        
        // Create output directory for this page
        let page_output_dir = self.output_dir.join("social").join(page_slug);
        fs::create_dir_all(&page_output_dir)?;
        
        // Generate all social media sizes
        for (name, logical_w, logical_h, densities) in SOCIAL_SPECS {
            for (suffix, scale) in densities.iter() {
                let width = (*logical_w as f32 * scale) as u32;
                let height = (*logical_h as f32 * scale) as u32;
                
                let filename = format!("{}{}.jpg", name, suffix);
                let output_path = page_output_dir.join(&filename);
                
                // Resize image
                let resized = img.resize_exact(width, height, FilterType::Lanczos3);
                
                // Save with high quality
                resized.save_with_format(&output_path, ImageFormat::Jpeg)?;
                
                // Store the path relative to static dir
                let relative_path = format!("/social/{}/{}", page_slug, filename);
                generated_images.insert(format!("{}_{}", name, suffix), relative_path);
                
                // Also generate Android-specific versions for mobile/tablet
                if name == &"mobile" || name == &"tablet" {
                    for (density_name, density_scale) in ANDROID_DENSITIES {
                        let android_width = (*logical_w as f32 * density_scale) as u32;
                        let android_height = (*logical_h as f32 * density_scale) as u32;
                        
                        let android_filename = format!("{}-{}.jpg", name, density_name);
                        let android_output_path = page_output_dir.join(&android_filename);
                        
                        let android_resized = img.resize_exact(android_width, android_height, FilterType::Lanczos3);
                        android_resized.save_with_format(&android_output_path, ImageFormat::Jpeg)?;
                        
                        let android_relative_path = format!("/social/{}/{}", page_slug, android_filename);
                        generated_images.insert(format!("{}_{}", name, density_name), android_relative_path);
                    }
                }
            }
        }
        
        // Generate meta tags data
        self.generate_meta_tags_data(&page_output_dir, page_slug, &generated_images)?;
        
        Ok(generated_images)
    }
    
    /// Generate a JSON file with all the meta tag data for templates
    fn generate_meta_tags_data(&self, output_dir: &Path, page_slug: &str, images: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let meta_data = serde_json::json!({
            "og_image": images.get("og_").unwrap_or(&String::new()),
            "og_image_1x": images.get("og_-1x").unwrap_or(&String::new()),
            "og_image_3x": images.get("og_-3x").unwrap_or(&String::new()),
            "og_image_square": images.get("og-square_").unwrap_or(&String::new()),
            "twitter_image": images.get("twitter_").unwrap_or(&String::new()),
            "linkedin_image": images.get("linkedin_").unwrap_or(&String::new()),
            "mobile_image": images.get("mobile_").unwrap_or(&String::new()),
            "tablet_image": images.get("tablet_").unwrap_or(&String::new()),
            "whatsapp_image": images.get("whatsapp_").unwrap_or(&String::new()),
            "pinterest_image": images.get("pinterest_").unwrap_or(&String::new()),
            "discord_image": images.get("discord_").unwrap_or(&String::new()),
            "instagram_story": images.get("instagram-story_").unwrap_or(&String::new()),
            "android_densities": {
                "mobile": {
                    "ldpi": images.get("mobile_ldpi").unwrap_or(&String::new()),
                    "mdpi": images.get("mobile_mdpi").unwrap_or(&String::new()),
                    "hdpi": images.get("mobile_hdpi").unwrap_or(&String::new()),
                    "xhdpi": images.get("mobile_xhdpi").unwrap_or(&String::new()),
                    "xxhdpi": images.get("mobile_xxhdpi").unwrap_or(&String::new()),
                    "xxxhdpi": images.get("mobile_xxxhdpi").unwrap_or(&String::new()),
                },
                "tablet": {
                    "ldpi": images.get("tablet_ldpi").unwrap_or(&String::new()),
                    "mdpi": images.get("tablet_mdpi").unwrap_or(&String::new()),
                    "hdpi": images.get("tablet_hdpi").unwrap_or(&String::new()),
                    "xhdpi": images.get("tablet_xhdpi").unwrap_or(&String::new()),
                    "xxhdpi": images.get("tablet_xxhdpi").unwrap_or(&String::new()),
                    "xxxhdpi": images.get("tablet_xxxhdpi").unwrap_or(&String::new()),
                }
            }
        });
        
        let meta_file = output_dir.join("meta.json");
        fs::write(meta_file, serde_json::to_string_pretty(&meta_data)?)?;
        
        Ok(())
    }
}

/// Generate Tera template function for social meta tags
pub fn generate_social_meta_macro() -> String {
    r#"
{% macro social_meta(page) %}
  {% set base_url = config.base_url | default(value="") %}
  {% set page_slug = page.slug | default(value="index") %}
  {% set social_base = base_url ~ "/social/" ~ page_slug %}
  
  {# Primary Open Graph tags #}
  <meta property="og:title" content="{{ page.title | default(value=config.title) }}">
  <meta property="og:description" content="{{ page.description | default(value=config.description) }}">
  <meta property="og:url" content="{{ page.permalink | default(value=base_url) }}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="{{ config.title }}">
  
  {# Open Graph Images - using @2x as default #}
  <meta property="og:image" content="{{ social_base }}/og.jpg">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="630">
  <meta property="og:image:type" content="image/jpeg">
  <meta property="og:image:alt" content="{{ page.title | default(value=config.title) }}">
  
  {# Additional image sizes for different contexts #}
  <meta property="og:image" content="{{ social_base }}/og-square.jpg">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="1200">
  
  {# Twitter Card tags #}
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:site" content="@kloudsamurai">
  <meta name="twitter:creator" content="@kloudsamurai">
  <meta name="twitter:title" content="{{ page.title | default(value=config.title) }}">
  <meta name="twitter:description" content="{{ page.description | default(value=config.description) }}">
  <meta name="twitter:image" content="{{ social_base }}/twitter.jpg">
  <meta name="twitter:image:alt" content="{{ page.title | default(value=config.title) }}">
  
  {# LinkedIn specific #}
  <meta property="og:image" content="{{ social_base }}/linkedin.jpg">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="627">
  
  {# WhatsApp #}
  <meta property="og:image" content="{{ social_base }}/whatsapp.jpg">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="630">
  
  {# Pinterest #}
  <meta property="og:image" content="{{ social_base }}/pinterest.jpg">
  <meta property="og:image:width" content="1000">
  <meta property="og:image:height" content="1500">
  
  {# Mobile optimized images #}
  <link rel="image_src" href="{{ social_base }}/mobile.jpg">
  
  {# Preload critical social images #}
  <link rel="preload" as="image" href="{{ social_base }}/og.jpg">
  
  {# Schema.org structured data #}
  <script type="application/ld+json">
  {
    "@context": "https://schema.org",
    "@type": "WebPage",
    "name": "{{ page.title | default(value=config.title) }}",
    "description": "{{ page.description | default(value=config.description) }}",
    "url": "{{ page.permalink | default(value=base_url) }}",
    "image": {
      "@type": "ImageObject",
      "url": "{{ social_base }}/og.jpg",
      "width": 2400,
      "height": 1260
    },
    "publisher": {
      "@type": "Organization",
      "name": "{{ config.title }}",
      "logo": {
        "@type": "ImageObject",
        "url": "{{ base_url }}/logo.png"
      }
    }
  }
  </script>
{% endmacro %}
"#.to_string()
}