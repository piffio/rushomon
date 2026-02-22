-- Link Tags - many-to-many relationship between links and tags
-- Tags are org-scoped free-form strings (spaces allowed, max 50 chars)

CREATE TABLE link_tags (
  link_id TEXT NOT NULL,
  tag_name TEXT NOT NULL,
  org_id TEXT NOT NULL,
  PRIMARY KEY (link_id, tag_name),
  FOREIGN KEY (link_id) REFERENCES links(id)
);

-- Efficient lookup of tags for a specific link
CREATE INDEX idx_link_tags_link_id ON link_tags(link_id);

-- Efficient lookup of all tags within an org (for autocomplete + counts)
CREATE INDEX idx_link_tags_org ON link_tags(org_id, tag_name);
