export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

    // Helper to get auth token
    const authHeader = request.headers.get("Authorization");
    const isAdmin = authHeader === `Bearer ${env.ADMIN_TOKEN}`;

    // CORS Headers
    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, PUT, PATCH, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type, Authorization",
    };

    if (request.method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders });
    }

    const apiIndex = path.indexOf("/api/links/");

    // 1. API ROUTES
    if (apiIndex !== -1) {
      const apiPath = path.substring(apiIndex);
      if (!isAdmin) {
        return new Response(JSON.stringify({ ok: false, error: "Unauthorized" }), {
          status: 401,
          headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }

      const parts = apiPath.substring("/api/links/".length).split("/");
      const slug = parts[0];
      const subRoute = parts[1]; // e.g. "tags", "current", or undefined

      const kvKey = `link:${slug}`;

      // GET /api/links/{slug} - Check if it exists or return config
      if (request.method === "GET" && !subRoute) {
        const data = await env.REDIRECTS_KV.get(kvKey);
        if (!data) {
          return new Response(JSON.stringify({ ok: false, error: "Not found" }), {
            status: 404,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        return new Response(data, {
          status: 200,
          headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }

      // PUT /api/links/{slug} - Create or update master link config
      if (request.method === "PUT" && !subRoute) {
        try {
          const body = await request.json();
          const { defaultUrl, ownerId, tags } = body;

          if (!defaultUrl) {
            return new Response(JSON.stringify({ ok: false, error: "Missing defaultUrl" }), {
              status: 400,
              headers: { ...corsHeaders, "Content-Type": "application/json" }
            });
          }

          // Fetch existing to preserve currentTag and currentSeq
          const existingData = await env.REDIRECTS_KV.get(kvKey);
          let currentTag = null;
          let currentUrl = null;
          let currentSeq = 0;

          if (existingData) {
            const parsed = JSON.parse(existingData);
            currentTag = parsed.currentTag || null;
            currentUrl = parsed.currentUrl || null;
            currentSeq = parsed.currentSeq || 0;
          }

          const newConfig = {
            defaultUrl,
            ownerId: ownerId || "default",
            tags: tags || {},
            currentTag,
            currentUrl,
            currentSeq
          };

          await env.REDIRECTS_KV.put(kvKey, JSON.stringify(newConfig));

          return new Response(JSON.stringify({ ok: true }), {
            status: 200,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        } catch (err) {
          return new Response(JSON.stringify({ ok: false, error: err.message }), {
            status: 500,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
      }

      // PATCH /api/links/{slug}/tags - Update tag mappings
      if (request.method === "PATCH" && subRoute === "tags") {
        try {
          const body = await request.json();
          const { tags } = body;

          const existingData = await env.REDIRECTS_KV.get(kvKey);
          if (!existingData) {
            return new Response(JSON.stringify({ ok: false, error: "Link not found" }), {
              status: 404,
              headers: { ...corsHeaders, "Content-Type": "application/json" }
            });
          }

          const config = JSON.parse(existingData);
          config.tags = tags || {};

          await env.REDIRECTS_KV.put(kvKey, JSON.stringify(config));

          return new Response(JSON.stringify({ ok: true }), {
            status: 200,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        } catch (err) {
          return new Response(JSON.stringify({ ok: false, error: err.message }), {
            status: 500,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
      }

      // PATCH /api/links/{slug}/current - Update active tag or direct url
      if (request.method === "PATCH" && subRoute === "current") {
        try {
          const body = await request.json();
          const { tag, url, seq } = body;

          const existingData = await env.REDIRECTS_KV.get(kvKey);
          if (!existingData) {
            return new Response(JSON.stringify({ ok: false, error: "Link not found" }), {
              status: 404,
              headers: { ...corsHeaders, "Content-Type": "application/json" }
            });
          }

          const config = JSON.parse(existingData);

          // Sequence checking (prevent stale out-of-order writes)
          if (seq !== undefined && config.currentSeq !== undefined && seq < config.currentSeq) {
            return new Response(JSON.stringify({ ok: true, ignored: true, reason: "Stale sequence number" }), {
              status: 200,
              headers: { ...corsHeaders, "Content-Type": "application/json" }
            });
          }

          if (tag !== undefined) {
            config.currentTag = tag;
            config.currentUrl = null; // Clear direct URL if tag is set
          } else if (url !== undefined) {
            config.currentUrl = url;
            config.currentTag = null; // Clear tag if direct URL is set
          }

          if (seq !== undefined) {
            config.currentSeq = seq;
          }

          await env.REDIRECTS_KV.put(kvKey, JSON.stringify(config));

          return new Response(JSON.stringify({ ok: true }), {
            status: 200,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        } catch (err) {
          return new Response(JSON.stringify({ ok: false, error: err.message }), {
            status: 500,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
      }
    }

    // 2. REDIRECTION (GET /{slug})
    if (request.method === "GET") {
      const segments = path.split("/").filter(Boolean);
      const slug = segments[segments.length - 1];
      if (!slug) {
        return new Response("Welcome to Tappy! (No slug specified)", { status: 200 });
      }

      const kvKey = `link:${slug}`;
      const data = await env.REDIRECTS_KV.get(kvKey);

      if (!data) {
        return new Response(`Shortlink "/${slug}" not found.`, { status: 404 });
      }

      try {
        const config = JSON.parse(data);
        let redirectUrl = config.defaultUrl;

        if (config.currentUrl) {
          redirectUrl = config.currentUrl;
        } else if (config.currentTag && config.tags && config.tags[config.currentTag]) {
          redirectUrl = config.tags[config.currentTag];
        }

        // Return a 302 redirect
        return Response.redirect(redirectUrl, 302);
      } catch (err) {
        return new Response(`Redirection failed: ${err.message}`, { status: 500 });
      }
    }

    return new Response("Not Found", { status: 404 });
  }
};
