# `github-mcp-server` MCP Search

## Last Updated: April 30th, 2025

## Step 1: Construct you1r Search Query Keywords

Think deeply and sequentially about the ideal terms that are not too specific, nor too generalized that will truly find what we're after, using the content and tags a programmer would apply to their github repository.

Use `in:readme` as your "go to" solution. you can also include `in:description` and `in:name` to capture more right on the nose matches.

## Step 2: Get Today's Date and Calculate "Recent" (6-months ago)

*Always* use the following bash command to get a minimum last_updated date. This is the date you should use for your search.

```bash
echo $(date -v-6m +"%Y-%m-%d")
```

### Example: "async runtime", updated in the last 6 months

```javascript
'async runtime' in:name in:description in:readme updated:>2024-10-30
```

## Step 3: Identify the lang filter

This will almost always be `lang:rust` unless David says otherwise.

### Example: Search for "async runtime", updated in the last 6 months, with lang:rust

```javascript
'async runtime' in:name in:description in:readme updated:>2024-10-30 lang:rust
```

## Step 4: Always sort by stars-desc

Combined with a recent cutoff date, this gets you into high quality results, quickly.

### Example: Search for "async runtime", updated in the last 6 months, with lang:rust, sort by stars-desc

```javascript
'async runtime' in:name in:description in:readme updated:>2024-10-30 lang:rust sort:stars-desc
```

## Step 5: Run the MCP tool

```javascript
mcp__github_mcp_server__search_repositories({
  query: "'async runtime' in:name in:description in:readme updated:>2024-10-30 lang:rust sort:stars-desc",
  perPage: 10,
  page: 1
});
```

## Step 6: Analyze the results

After finding repositories with the search, you would use the `get_file_contents` tool to analyze the results.

Start by pulling the README.md file. Skim it and see if it's a good fit for your project. If it is, read it in full and further establish if it's a great fit.

```javascript
mcp__github_mcp_server__get_file_contents({
  owner: "cyrup-ai", // owner from search results
  repo: "sweet_mcp_server", // repo from search results
  path: "README.md",
  branch: "main" // main-branch from search results
});
```

It's often helpful to look at the Cargo.toml file as well to see what libraries the results are utilizing, the license structure and other metadata.

```javascript
mcp__github_mcp_server__get_file_contents({
  owner: "cyrup-ai", // owner from search results
  repo: "sweet_mcp_server", // repo from search results
  path: "Cargo.toml",
  branch: "main" // main-branch from search results
});
```

Think about what we're researching sequentially and deeply, step by step vs. the README.md and Cargo.toml content. Does this result really satisfy the core search criteria? Is the "the one"? If not, move on through the results.

If you go through 10 and none are making the grade, try refining your keywords, repeating steps 1-6, until you find the a "great result" for your query.

If you think this is the candidate and want to see source code for it, you can use the `get_file_contents` tool to get the source code.

```javascript
mcp__github_mcp_server__get_file_contents({
  owner: "cyrup-ai", // owner from search results
  repo: "sweet_mcp_server", // repo from search results
  path: "src/main.rs",
  branch: "main" // main-branch from search results
});
```

## Step 7: Report on your Findings

After analyzing the repository, report back on your findings.

### If you found a candidate

- Mention a sampling of the best result repositories you analyzed.
- Tell the user the one you recommend, and why it is differentiated from the other results. **"We will be able to use X in our project, to accomplish Y ..."**

### If no candidate found

- Reflect deeply. Think sequentially about the query and steps used to search.
- Were they too narrow? Too broad? Off target?
- If yes, think outloud about what went wrong and how you can improve the search. Then repeat steps 1-6.
- If no, reflect on the overall goal and think outloud about a different approach than github search.
- Always ask the user "Do you have any questions about these results?"

### General Notes for Reporting Github Search Results

- *Always* include the stargazers count and the last pushed date for each repository you speak about.
- *Never* recommend repositories with only a handful of stars or which haven't been updated in years.
