use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;
use walkdir::WalkDir;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=content/");
    println!("cargo:rerun-if-changed=static/assets/img/");
    
    // Process all content files and generate social images
    process_content_files();
    
    // Generate the social meta template
    generate_social_meta_template();
}

fn process_content_files() {
    let content_dir = Path::new("content");
    if !content_dir.exists() {
        return;
    }
    
    // Walk through all markdown files
    for entry in WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let path = entry.path();
        if let Ok(content) = fs::read_to_string(path) {
            // Parse frontmatter
            if let Some(frontmatter) = extract_frontmatter(&content) {
                process_page_frontmatter(path, &frontmatter);
            }
        }
    }
}

fn extract_frontmatter(content: &str) -> Option<Value> {
    // Check if content starts with +++
    if !content.starts_with("+++") {
        return None;
    }
    
    // Find the closing +++
    let parts: Vec<&str> = content.splitn(3, "+++").collect();
    if parts.len() < 3 {
        return None;
    }
    
    // Parse TOML frontmatter
    toml::from_str(parts[1]).ok()
}

fn process_page_frontmatter(path: &Path, frontmatter: &Value) {
    // Extract page slug from path
    let slug = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("index");
    
    // Check for og_image in frontmatter
    let og_image = if let Some(explicit_image) = frontmatter.get("og_image").and_then(|v| v.as_str()) {
        // Use explicitly specified image
        println!("cargo:warning=Processing explicit og_image for {}: {}", slug, explicit_image);
        explicit_image.to_string()
    } else {
        // No explicit og_image - select deterministic random from banner pool based on slug
        match select_random_banner(slug) {
            Some(banner) => {
                println!("cargo:warning=Using auto-selected banner for {}: {}", slug, banner);
                banner
            }
            None => {
                println!("cargo:warning=No og_image or banners found for {}", slug);
                return;
            }
        }
    };
    
    // Generate all social media sizes for this page
    if let Err(e) = generate_page_social_images(slug, &og_image) {
        println!("cargo:warning=Failed to generate social images for {}: {}", slug, e);
    }
}

fn generate_page_social_images(slug: &str, og_image_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use image::{DynamicImage, ImageFormat, imageops::FilterType};
    
    let source_path = if og_image_path.starts_with("/") {
        PathBuf::from("static").join(&og_image_path[1..])
    } else {
        PathBuf::from("static").join(og_image_path)
    };
    
    if !source_path.exists() {
        return Err(format!("Source image not found: {:?}", source_path).into());
    }
    
    let img = image::open(&source_path)?;
    
    // Create output directory for this page
    let output_dir = PathBuf::from("static/social").join(slug);
    fs::create_dir_all(&output_dir)?;
    
    // Social media specifications
    let specs = vec![
        // Platform, base_name, width, height, densities
        ("og", 1200, 630, vec![("1x", 1.0), ("", 2.0), ("3x", 3.0)]),
        ("og-square", 1200, 1200, vec![("1x", 1.0), ("", 2.0)]),
        ("twitter", 1200, 675, vec![("1x", 1.0), ("", 2.0)]),
        ("linkedin", 1200, 627, vec![("1x", 1.0), ("", 2.0)]),
        ("whatsapp", 1200, 630, vec![("1x", 1.0), ("", 2.0)]),
        ("pinterest", 1000, 1500, vec![("1x", 1.0), ("", 2.0)]),
        ("discord", 1200, 630, vec![("1x", 1.0), ("", 2.0)]),
        ("instagram-story", 1080, 1920, vec![("1x", 1.0), ("", 2.0)]),
        ("mobile", 1080, 1920, vec![("1x", 1.0), ("", 2.0), ("3x", 3.0)]),
        ("tablet", 1920, 1200, vec![("1x", 1.0), ("", 2.0)]),
    ];
    
    // Generate each size
    for (name, base_w, base_h, densities) in specs {
        for (suffix, scale) in densities {
            let width = (base_w as f32 * scale) as u32;
            let height = (base_h as f32 * scale) as u32;
            
            let filename = if suffix.is_empty() {
                format!("{}.jpg", name)
            } else {
                format!("{}-{}.jpg", name, suffix)
            };
            
            let output_path = output_dir.join(&filename);
            
            // Resize and save
            let resized = img.resize_exact(width, height, FilterType::Lanczos3);
            resized.save_with_format(&output_path, ImageFormat::Jpeg)?;
            
            println!("cargo:warning=  Generated {}/{} ({}x{})", slug, filename, width, height);
        }
        
        // Generate Android density versions for mobile/tablet
        if name == "mobile" || name == "tablet" {
            let android_densities = vec![
                ("ldpi", 0.75),
                ("mdpi", 1.0),
                ("hdpi", 1.5),
                ("xhdpi", 2.0),
                ("xxhdpi", 3.0),
                ("xxxhdpi", 4.0),
            ];
            
            for (density_name, density_scale) in android_densities {
                let width = (base_w as f32 * density_scale) as u32;
                let height = (base_h as f32 * density_scale) as u32;
                
                let filename = format!("{}-{}.jpg", name, density_name);
                let output_path = output_dir.join(&filename);
                
                let resized = img.resize_exact(width, height, FilterType::Lanczos3);
                resized.save_with_format(&output_path, ImageFormat::Jpeg)?;
            }
        }
    }
    
    Ok(())
}

fn select_random_banner(slug: &str) -> Option<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Look for banner images in multiple locations
    let banner_patterns = vec![
        "static/assets/img/banners/*.jpg",
        "static/assets/img/banners/*.png",
        "static/assets/img/banners/*.gif",
        "static/assets/img/social-defaults/*.jpg",
        "static/assets/img/social-defaults/*.png",
        "static/assets/img/social-defaults/*.gif",
        "static/assets/img/og-defaults/*.jpg",
        "static/assets/img/og-defaults/*.png",
        "static/assets/img/og-defaults/*.gif",
        // Also check hero images as fallback
        "static/assets/img/hero/*.jpg",
        "static/assets/img/hero/*.png",
        "static/assets/img/hero/*.gif",
    ];
    
    let mut all_banners = Vec::new();
    
    for pattern in &banner_patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for path in paths.flatten() {
                if let Ok(relative) = path.strip_prefix("static") {
                    all_banners.push(format!("/{}", relative.display()));
                }
            }
        }
    }
    
    if all_banners.is_empty() {
        return None;
    }
    
    // Sort for consistency
    all_banners.sort();
    
    // Use deterministic selection based on page slug
    // This ensures the same page always gets the same banner
    // (unless the banner pool changes)
    let mut hasher = DefaultHasher::new();
    slug.hash(&mut hasher);
    let hash = hasher.finish();
    
    let index = (hash as usize) % all_banners.len();
    Some(all_banners[index].clone())
}

fn generate_social_meta_template() {
    let template_dir = Path::new("templates/macros");
    fs::create_dir_all(template_dir).expect("Failed to create template directory");
    
    let social_meta_template = r#"{% macro social_meta(page) %}
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
  
  {# Pinterest Rich Pins #}
  <meta property="og:image" content="{{ social_base }}/pinterest.jpg">
  <meta property="og:image:width" content="1000">
  <meta property="og:image:height" content="1500">
  <meta name="pinterest:media" content="{{ social_base }}/pinterest.jpg">
  <meta name="pinterest:description" content="{{ page.description | default(value=config.description) }}">
  
  {# Discord #}
  <meta property="og:image" content="{{ social_base }}/discord.jpg">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="630">
  
  {# Mobile optimized images #}
  <link rel="image_src" href="{{ social_base }}/mobile.jpg">
  
  {# Responsive images for different devices #}
  <meta name="mobile-web-app-capable" content="yes">
  <meta name="apple-mobile-web-app-capable" content="yes">
  <link rel="apple-touch-icon" sizes="1200x1200" href="{{ social_base }}/og-square.jpg">
  
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
{% endmacro %}"#;
    
    let template_path = template_dir.join("social_meta.html");
    fs::write(template_path, social_meta_template).expect("Failed to write social meta template");
}

fn update_social_sizes_docs() {
    let docs_content = r#"# Social Media Image Size Guide

This guide provides the correct image sizes for social media sharing across different platforms, including pixel density considerations for high-resolution displays.

## Understanding Pixel Density

### Device Pixel Ratios (DPR)
- **@1x** = Standard displays (DPR 1.0) - ldpi/mdpi Android
- **@2x** = Retina displays (DPR 2.0) - hdpi/xhdpi Android, Retina iOS
- **@3x** = Super Retina (DPR 3.0) - xxhdpi Android, iPhone Plus/Pro
- **@4x** = Ultra high density (DPR 4.0) - xxxhdpi Android

### Android Density Buckets
- **ldpi** (~120dpi) - 0.75x - Low density
- **mdpi** (~160dpi) - 1.0x - Medium density (baseline)
- **hdpi** (~240dpi) - 1.5x - High density
- **xhdpi** (~320dpi) - 2.0x - Extra high density
- **xxhdpi** (~480dpi) - 3.0x - Extra extra high density
- **xxxhdpi** (~640dpi) - 4.0x - Extra extra extra high density

### Key Concepts
- **Logical Pixels**: The size specified in meta tags (e.g., 1200x630)
- **Physical Pixels**: The actual image dimensions (e.g., 2400x1260 for @2x)
- **Best Practice**: Upload @2x images minimum, platforms will downscale as needed

## Primary Open Graph Size (Universal)

**Logical Size**: 1200x630 pixels (1.91:1 aspect ratio)
**Actual Image Sizes to Generate**:
- **@1x**: 1200x630 pixels (standard displays)
- **@2x**: 2400x1260 pixels (retina displays) **← RECOMMENDED UPLOAD SIZE**
- **@3x**: 3600x1890 pixels (super retina, optional)

## Platform-Specific Requirements

### Facebook Open Graph
- **Logical Size**: 1200x630 pixels
- **Minimum**: 600x315 pixels
- **Recommended Upload**: 2400x1260 pixels (@2x)
- **Aspect Ratio**: 1.91:1
- **Max File Size**: 8MB
- **Format**: JPG or PNG

### Twitter/X Cards
- **Summary Card Large Image**: 
  - Logical: 1200x675 pixels
  - Upload: 2400x1350 pixels (@2x)
- **Summary Card (shown cropped)**: 
  - Display: 508x266 pixels (2:1.04 ratio)
  - Note: Twitter crops all shared links to this size
- **In-Feed Images**:
  - Landscape: 1200x628 pixels
  - Square: 1200x1200 pixels
  - Vertical: 720x900 pixels
- **Max File Size**: ~2MB
- **Format**: JPG, PNG, or GIF

### LinkedIn
- **Link Preview Images**:
  - Logical: 1200x627 pixels
  - Upload: 2400x1254 pixels (@2x)
- **Profile Photo**: 400x400 pixels (will be cropped to circle)
- **Cover Image**: 1128x191 pixels
- **Company Logo**: 300x300 pixels
- **Max File Size**: 8MB
- **Format**: JPG or PNG

### Mobile & Tablet Specific

#### iOS Devices
- **iPhone SE/8**: 375x667 logical, 750x1334 @2x
- **iPhone 14/15**: 390x844 logical, 1170x2532 @3x
- **iPhone 14/15 Pro Max**: 430x932 logical, 1290x2796 @3x
- **iPad**: 768x1024 logical, 1536x2048 @2x
- **iPad Pro 12.9"**: 1024x1366 logical, 2048x2732 @2x

#### Android Devices
- **Common Phone**: 360x640 logical, varies by density
- **Pixel 7**: 412x915 logical, 1080x2400 physical (~2.6x)
- **Samsung Galaxy S23**: 360x780 logical, 1080x2340 physical (3x)
- **Common Tablet**: 600x960 logical, varies by density
- **Samsung Galaxy Tab**: 800x1280 logical, 2560x1600 physical

## Image Generation Requirements

### For sweet.async.wiki, generate these sizes:

1. **Primary Open Graph Set**:
   - `og-image.jpg` - 2400x1260px (@2x for 1200x630 logical)
   - `og-image-1x.jpg` - 1200x630px (fallback)
   - `og-image-3x.jpg` - 3600x1890px (optional ultra-high)

2. **Square Format Set**:
   - `og-image-square.jpg` - 2400x2400px (@2x for 1200x1200 logical)
   - `og-image-square-1x.jpg` - 1200x1200px (fallback)

3. **Twitter Optimized Set**:
   - `twitter-card.jpg` - 2400x1350px (@2x for 1200x675 logical)
   - `twitter-card-1x.jpg` - 1200x675px (fallback)

4. **LinkedIn Specific**:
   - `linkedin-share.jpg` - 2400x1254px (@2x for 1200x627 logical)
   - `linkedin-share-1x.jpg` - 1200x627px (fallback)

5. **Mobile/Tablet Optimized**:
   - `og-image-mobile.jpg` - 2160x3840px (@2x for 1080x1920 logical)
   - `og-image-tablet.jpg` - 3840x2400px (@2x for 1920x1200 logical)

### Meta Tag Implementation

```html
<!-- Specify logical pixels in meta tags -->
<meta property="og:image" content="/assets/img/og-image.jpg">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:type" content="image/jpeg">

<!-- The actual og-image.jpg should be 2400x1260 pixels -->
```

## Image Processing with Rust

### Using the `image` crate:
```rust
use image::{DynamicImage, ImageBuffer, Rgb};

// Load and resize for different densities
let img = image::open("source.jpg")?;

// Generate @2x version (recommended default)
let img_2x = img.resize(2400, 1260, image::imageops::FilterType::Lanczos3);
img_2x.save("og-image.jpg")?;

// Generate @1x fallback
let img_1x = img.resize(1200, 630, image::imageops::FilterType::Lanczos3);
img_1x.save("og-image-1x.jpg")?;
```

### Using `resvg` for SVG to raster:
```rust
use resvg::usvg::{self, TreeParsing};
use resvg::tiny_skia;

// Render SVG at different densities
let opt = usvg::Options::default();
let tree = usvg::Tree::from_str(&svg_data, &opt)?;

// Render @2x
let pixmap = tiny_skia::Pixmap::new(2400, 1260)?;
resvg::render(&tree, usvg::Transform::default(), &mut pixmap.as_mut());
```

## Best Practices

1. **Always upload @2x minimum**: Ensures crisp display on retina devices
2. **Optimize file size**: Use proper compression (85-90% JPEG quality)
3. **Test on real devices**: Check how images appear on various phones/tablets
4. **Safe area**: Keep important content away from edges (15% margin)
5. **Text legibility**: Any text should be readable even at smaller display sizes
6. **Fallback images**: Provide @1x versions for bandwidth-constrained situations
7. **Android considerations**: Test on both high-end (3x-4x) and low-end (1x-1.5x) devices

## Current Site Images Status

- [ ] `og-image.jpg` - Need to create at 2400x1260px (@2x)
- [ ] `og-image-square.jpg` - Need to create at 2400x2400px (@2x)
- [ ] `twitter-card.jpg` - Need to create at 2400x1350px (@2x)
- [ ] `linkedin-share.jpg` - Need to create at 2400x1254px (@2x)
- [ ] `og-image-mobile.jpg` - Need to create at 2160x3840px (@2x)
- [ ] `og-image-tablet.jpg` - Need to create at 3840x2400px (@2x)
- [x] `hero.png` - Full-width hero background (varies by screen)
- [x] Numbered images (1.png, 2.jpg, 3.gif, 4.jpg) - Hero rotation images
"#;

    let docs_path = Path::new("static/assets/img/SOCIAL-MEDIA-SIZES.md");
    fs::write(docs_path, docs_content).expect("Failed to update social sizes documentation");
}