import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  workspaces: defineTable({
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
  }).index("by_slug", ["slug"]),

  status: defineTable({
    slug: v.string(),
    status: v.string(),
    updated_at: v.string(),
    agent: v.string(),
    needs_attention: v.boolean(),
    pr_number: v.optional(v.number()),
  }).index("by_slug", ["slug"]),
});
