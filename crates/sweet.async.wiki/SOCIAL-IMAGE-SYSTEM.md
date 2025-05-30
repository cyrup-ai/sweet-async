# Social Image Generation System

This documentation explains how the Sweet Async Wiki automatically generates all required social media images from a single source image specified in each page's frontmatter.

## How It Works

### 1. Page Configuration

Each markdown page can either specify its own `og_image` OR let the system auto-select one:

#### Option A: Explicit Image (Recommended for key pages)
```toml
+++
title = "Your Page Title"
description = "Your page description"
og_image = "/assets/img/your-image.jpg"  # Explicitly chosen image
hero_image = "/assets/img/hero/2.jpg"   # Optional: different hero image
+++
```

#### Option B: Auto-Selected Image (Great for blog posts, docs)
```toml
+++
title = "Your Page Title"
description = "Your page description"
# No og_image specified - system will auto-select:
# 1. First tries Unsplash semantic search (if API key set)
# 2. Falls back to random selection from banner pool
+++
```

#### Option C: Semantic Hints for Better Unsplash Results
```toml
+++
title = "Building Distributed Systems"
description = "Learn how to build scalable distributed systems"
tags = ["cloud", "architecture", "servers"]
keywords = "distributed, systems, cloud, computing"
category = "technology"
# System will search Unsplash for: "Building Distributed Systems cloud architecture servers"
+++
```

### 2. Automatic Generation

The build system (build.rs) automatically:
- Scans all markdown files in `content/`
- Extracts the `og_image` property from frontmatter
- Generates 50+ image variations for each page
- Saves them to `/static/social/{page-slug}/`
- Creates proper meta tag templates

### 3. Generated Images Per Page

For each page with an `og_image`, the system generates:

#### Standard Social Platforms
- **Open Graph** (Facebook, general sharing):
  - `og.jpg` - 2400x1260 (@2x default)
  - `og-1x.jpg` - 1200x630 (@1x fallback)
  - `og-3x.jpg` - 3600x1890 (@3x ultra-high)

- **Square Format** (Twitter summary, Instagram):
  - `og-square.jpg` - 2400x2400 (@2x default)
  - `og-square-1x.jpg` - 1200x1200 (@1x fallback)

- **Twitter Cards**:
  - `twitter.jpg` - 2400x1350 (@2x default)
  - `twitter-1x.jpg` - 1200x675 (@1x fallback)

- **LinkedIn**:
  - `linkedin.jpg` - 2400x1254 (@2x default)
  - `linkedin-1x.jpg` - 1200x627 (@1x fallback)

- **WhatsApp**:
  - `whatsapp.jpg` - 2400x1260 (@2x default)
  - `whatsapp-1x.jpg` - 1200x630 (@1x fallback)

- **Pinterest**:
  - `pinterest.jpg` - 2000x3000 (@2x default)
  - `pinterest-1x.jpg` - 1000x1500 (@1x fallback)

- **Discord**:
  - `discord.jpg` - 2400x1260 (@2x default)
  - `discord-1x.jpg` - 1200x630 (@1x fallback)

- **Instagram Story**:
  - `instagram-story.jpg` - 2160x3840 (@2x default)
  - `instagram-story-1x.jpg` - 1080x1920 (@1x fallback)

#### Mobile & Tablet Optimized

- **Mobile** (portrait):
  - `mobile.jpg` - 2160x3840 (@2x default)
  - `mobile-1x.jpg` - 1080x1920 (@1x)
  - `mobile-3x.jpg` - 3240x5760 (@3x)
  - Android densities: `mobile-{ldpi,mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}.jpg`

- **Tablet** (landscape):
  - `tablet.jpg` - 3840x2400 (@2x default)
  - `tablet-1x.jpg` - 1920x1200 (@1x)
  - Android densities: `tablet-{ldpi,mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}.jpg`

### 4. Meta Tag Implementation

The system automatically generates a `social_meta.html` template that includes:

```html
<!-- Primary Open Graph -->
<meta property="og:image" content="/social/page-slug/og.jpg">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">

<!-- Twitter Cards -->
<meta name="twitter:image" content="/social/page-slug/twitter.jpg">

<!-- Platform-specific images -->
<meta property="og:image" content="/social/page-slug/linkedin.jpg">
<!-- ... and many more -->
```

## Source Image Requirements

### Recommended Specifications
- **Minimum Size**: 3840x2400px (to support all formats)
- **Ideal Size**: 4000x4000px or larger (square, will be cropped)
- **Format**: JPG, PNG, or GIF (animated GIFs supported)
- **Quality**: High quality, uncompressed
- **File Size**: No specific limit (will be optimized during generation)

### Content Guidelines
1. **Safe Area**: Keep important content centered (15% margin from edges)
2. **Text**: Any text should be large and readable
3. **Contrast**: High contrast for visibility on various backgrounds
4. **Branding**: Include your logo/branding elements

## Auto-Selection Strategies

### 1. Unsplash Semantic Search (Requires API Key)

When `UNSPLASH_ACCESS_KEY` environment variable is set, the system:

1. **Builds semantic query** from page metadata:
   - Extracts keywords from title (excluding common words)
   - Includes tags, keywords, and category if present
   - Creates intelligent search query (e.g., "distributed systems cloud")

2. **Fetches relevant images** from Unsplash:
   - Searches for landscape-oriented photos
   - Uses deterministic selection (same page = same image)
   - Caches images locally to avoid repeated API calls
   - Stores attribution information

3. **Cache location**:
   ```
   static/assets/img/unsplash-cache/
   ├── page-slug.jpg              # Cached image
   └── page-slug.attribution.txt  # Photo credits
   ```

### 2. Local Banner Pool Fallback

If Unsplash is not available or returns no results, the system selects from local banner pools:

```
static/assets/img/
├── banners/          # Primary banner pool
│   ├── banner1.jpg
│   ├── banner2.png
│   └── banner3.gif
├── social-defaults/  # Social-specific defaults
│   ├── default1.jpg
│   └── default2.png
├── og-defaults/      # OpenGraph-specific defaults
│   └── og-banner.jpg
└── hero/            # Falls back to hero images
    ├── 1.png
    ├── 2.jpg
    └── 3.gif
```

### How Auto-Selection Works

1. **Deterministic**: Each page slug generates a hash that selects the same banner consistently
2. **Automatic**: Just add images to any banner directory
3. **Flexible**: Supports JPG, PNG, and GIF (including animated)
4. **Fallback**: Checks multiple directories in order

### Example:
- Page `docs/api.md` → Always selects the same banner (e.g., `banner2.png`)
- Page `blog/update.md` → Always selects a different banner (e.g., `banner5.jpg`)
- Ensures visual variety while maintaining consistency

## Build Process

When you run the build:

```bash
# Optional: Set Unsplash API key for semantic image search
export UNSPLASH_ACCESS_KEY="your-unsplash-access-key"

cargo build
```

The build.rs script:
1. Scans all `.md` files in `content/`
2. For each page:
   - If `og_image` is specified → use that image
   - If not specified:
     - Try Unsplash semantic search (if API key set)
     - Build query from title, tags, keywords, category
     - Cache successful downloads
     - Fall back to local banner pool if needed
3. Generates all image sizes using the `image` crate
4. Uses Lanczos3 filtering for high-quality resizing
5. Saves optimized JPEGs at 90% quality
6. Creates the social_meta.html template

## Usage Example

### 1. Create your page:

```markdown
+++
title = "Orchestrator is an AsyncTask"
description = "The fundamental philosophy of Sweet Async"
og_image = "/assets/img/orchestrator-hero.png"
+++

# Your content here...
```

### 2. Add the source image:

Place your high-res image at:
```
static/assets/img/orchestrator-hero.png
```

### 3. Build generates:

```
static/social/orchestrator/
├── og.jpg (2400x1260)
├── og-1x.jpg (1200x630)
├── og-3x.jpg (3600x1890)
├── twitter.jpg (2400x1350)
├── linkedin.jpg (2400x1254)
├── mobile-xxhdpi.jpg (3240x5760)
└── ... (50+ more files)
```

### 4. Templates use them:

In your base template:
```html
{% include "macros/social_meta.html" %}
{{ social_meta(page=page) }}
```

## Benefits

1. **Automatic**: No manual image resizing needed
2. **Comprehensive**: Every platform covered
3. **Optimized**: Proper density for all devices
4. **SEO-friendly**: All meta tags properly configured
5. **Performant**: Images generated at build time
6. **Flexible**: Each page can have unique social images
7. **Zero-config option**: Pages without og_image still get beautiful social cards
8. **Consistent randomness**: Same page always gets same auto-selected banner
9. **Semantic image matching**: Unsplash integration finds contextually relevant images
10. **Attribution tracking**: Automatically stores photo credits for Unsplash images

## Troubleshooting

### Image not generating?
- Check that `og_image` path starts with `/` and points to a file in `static/`
- Ensure source image exists before building
- Check build output for warnings

### Wrong aspect ratio?
- The system crops to fit each platform's requirements
- Use a square source image (4000x4000) for best results
- Keep important content centered

### Unsplash not finding good matches?
- Add more specific tags to your frontmatter
- Use the `keywords` field for additional context
- Check the generated query in build output
- Ensure your UNSPLASH_ACCESS_KEY is valid

### Want to see Unsplash attribution?
- Check `static/assets/img/unsplash-cache/*.attribution.txt`
- Contains photographer name, username, and image URL

### File sizes too large?
- The system optimizes to 90% JPEG quality
- Consider using a smaller source image if needed
- PNG sources will be converted to JPEG for smaller sizes