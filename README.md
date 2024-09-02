# hugo-ai

*DANGER* This might delete some parts of your blog posts. Do not use without a very good backup.

AI tools for the [Hugo](https://gohugo.io/) static site generator.

*Calculate related posts*: This uses an OpenAI model to calculate an embedding (a vector representation of your blog post in latent space) for each post, and then finds other posts that are near it in that latent space. Those are similar. It then writes that into your blog's markdown files so you can do "What to read next" or "Similar posts" or "You might also like", that kind of thing.

*Summarize*: Generate a one-paragraph summary of the post and add it to the front-matter as `synopsis` field. It doesn't use "summary" because that already means something to Hugo.

## Prerequisites

- Install sqlite
- Backup your blog's markdown files. I store mine in a git repo, and recommend that. But back them up, really.
- Get an OpenAI API key
- Read Simon Willison's [Understand embeddings](https://simonwillison.net/2023/Oct/23/embeddings/). You don't have to do this at all, but it is really interesting.

## Install

No releases yet so you have to build from the Rust source. `cargo build --release` in project root should do it. It puts the binary in `target/release/hugo-ai`.

# Similar posts

## Generate similar / related posts

The tool proceeds in four careful steps. You can safely re-run any of these steps and it will only work on new content.

1. `hugo-ai similar gather my-blog/contents/posts`

This reads all the Markdown files in that directory, splits them up into reasonable sized chunks, and stores the information in a sqlite database.

The database defaults to `.config/hugo-ai/hugo-ai.db`. You can override with `--db-path <path>`.

2. `hugo-ai similar embed`

First set environment variable OPENAI_API_KEY to your key: `export OPENAI_API_KEY=<here>`

This takes all the article chunks from step 1, sends them to OpenAI's `text-embedding-3-small` model to calculate an *embedding* (a vector of 1,536 floating point numbers), and stores that back in the database.

This is the slowest step, and it costs you money. For the 270 articles on my blog it costs significantly less than $0.01. Not a typo. The [OpenAI pricing page](https://platform.openai.com/docs/guides/embeddings/use-cases) says to spend a whole dollar you would need to embed over 60,000 pages.

3. `hugo-ai similar calc`

Calculate a value for how similar each article is to every other one. Store that in the sqlite database too. It uses cosine similarity to compare the embedding vectors. In my experience it works really well. You'll be amazed. The `similarity` score is a floating point number between 0 and 1. Higher is more similar.

4. `hugo-ai similar write my-blog/content/posts [--no-backup] [--dry-run]`

Write out related posts into your markdown files. It adds something like this to your front-matter (the top part between dashes):
```
related:
- 2022-08-31-underrust.md
- 2022-09-02-underrust-types.md
- 2022-02-26-return-value-optimization-in-rust.md
```

These are the (up to) three most similar articles. If fewer than three articles meet the similarty threshold, then fewer are written.

By default it copies every post to a `.BAK`, out of an abundance of caution. If your posts are in git then add `--no-backup` to avoid that.

Adding `--dry-run` writes out the modified post to stdout instead of editing the files. That allows you to sanity check what you'll get.

## Display the similar posts

Now that you have the data, you need to edit your hugo template to display it.

I chose to put the related posts at the bottom of each individual post. I edited `themes/<theme-name>/layouts/defaults/single.html` and at the bottom in the `<footer>` I added
```
      {{ partial "related.html" . }}
```

Then in `themes/<theme>/layouts/partials` I created `related.html` like this:
```
<hr />
<div class="pt-3">
        <ul>
        {{ range .Params.related }}
                {{ $related := site.GetPage . }}
                {{ partial "article-link.html" $related }}
        {{ end }}
        </ul>
</div>
```

Hopefully that's sufficient to get you going.

# Summaries

Set environment variable OPENAI_API_KEY to your key: `export OPENAI_API_KEY=<here>`

Generate: `hugo-ai summary ~/src/my-blog/content/posts/`

This takes a while. It makes a call to OpenAI for every post. It uses the 4o-mini model to avoid usage limits, and because I found it produces very similar summaries to the big model. Summarizing my entire blog costs me less than $2.

It overwrites your Markdown file, adding a `synopsis` field.

Edit your `themes/<theme>/layouts/_default/single.html` and add the summary. HTML has a very nice element specifically for this:
```
{{ with .Params.Synopsis }}
    <details id="synopsis">
            <summary>Summary</summary>
            <div>{{ . }}</div>
    </details>
{{- end }}
```

---

All of this was inspired by [Simon Willison doing it here](https://simonwillison.net/2023/Oct/23/embeddings/#related-content-using-embeddings) for his blog.

