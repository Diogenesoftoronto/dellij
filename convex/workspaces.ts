import { mutation, query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
  handler: async (ctx) => {
    return await ctx.db.query("workspaces").order("desc").collect();
  },
});

export const upsert = mutation({
  args: {
    slug: v.string(),
    prompt: v.string(),
    agent: v.string(),
    branch_name: v.string(),
    base_branch: v.string(),
    worktree_path: v.string(),
    status: v.string(),
    updated_at: v.string(),
    pr_number: v.optional(v.number()),
    pr_url: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const existing = await ctx.db
      .query("workspaces")
      .withIndex("by_slug", (q) => q.eq("slug", args.slug))
      .unique();

    if (existing) {
      await ctx.db.patch(existing._id, args);
    } else {
      await ctx.db.insert("workspaces", args);
    }
  },
});
