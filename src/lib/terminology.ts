/**
 * Database-agnostic terminology system.
 *
 * This module provides terminology abstraction for different database paradigms,
 * enabling the UI, help content, and AI prompts to use appropriate terms.
 *
 * Example: "Table" (SQL) vs "Collection" (MongoDB) vs "Bucket" (Redis)
 */

import type { DatabaseParadigm } from "@/types/schema";

// Re-export for convenience
export type { DatabaseParadigm };

/**
 * Singular and plural forms of a term.
 */
export interface TermPair {
  singular: string;
  plural: string;
}

/**
 * Complete terminology mapping for a database paradigm.
 */
export interface Terminology {
  // Core nouns (singular/plural)
  /** Container for data (Table, Collection, Bucket, etc.) */
  container: TermPair;
  /** Individual data item (Row, Document, Entry, etc.) */
  item: TermPair;
  /** Data attribute (Column, Field, Key, etc.) */
  attribute: TermPair;
  /** Reference to other data (Foreign Key, Reference, Edge, etc.) */
  reference: TermPair;
  /** Data constraint (Constraint, Validation Rule, etc.) */
  constraint: TermPair;
  /** Schema/namespace (Schema, Database, Namespace, etc.) */
  schema: TermPair;

  // Compound terms
  /** Count of items (Row Count, Document Count, etc.) */
  itemCount: string;
  /** List of attributes (Columns, Fields, etc.) */
  attributeList: string;
  /** List of references (Foreign Keys, References, etc.) */
  referenceList: string;

  // Verbs/Actions
  actions: {
    /** Query data (Query, Find, Get, Match, etc.) */
    query: string;
    /** Insert data (Insert, Create, Set, etc.) */
    insert: string;
    /** Update data (Update, Set, etc.) */
    update: string;
    /** Delete data (Delete, Remove, etc.) */
    delete: string;
  };

  // Help text fragments
  help: {
    /** Description of what a container holds */
    containerDescription: string;
    /** Description of what an attribute defines */
    attributeDescription: string;
    /** Description of what a reference does */
    referenceDescription: string;
    /** Empty state message for no containers */
    noContainers: string;
    /** Empty state message for no attributes */
    noAttributes: string;
    /** Empty state message for no references */
    noReferences: string;
  };
}

/**
 * Terminology mappings for all supported database paradigms.
 */
export const TERMINOLOGY: Record<DatabaseParadigm, Terminology> = {
  relational: {
    container: { singular: "Table", plural: "Tables" },
    item: { singular: "Row", plural: "Rows" },
    attribute: { singular: "Column", plural: "Columns" },
    reference: { singular: "Foreign Key", plural: "Foreign Keys" },
    constraint: { singular: "Constraint", plural: "Constraints" },
    schema: { singular: "Schema", plural: "Schemas" },
    itemCount: "Row Count",
    attributeList: "Columns",
    referenceList: "Foreign Keys",
    actions: {
      query: "Query",
      insert: "Insert",
      update: "Update",
      delete: "Delete",
    },
    help: {
      containerDescription:
        "Tables store structured data with defined columns and relationships.",
      attributeDescription:
        "Columns define the structure of your table data. Each column has a type, constraints, and optional default values.",
      referenceDescription:
        "Foreign keys create relationships between tables, linking rows across the database.",
      noContainers: "No tables found",
      noAttributes: "No columns found",
      noReferences: "No relationships found",
    },
  },

  document: {
    container: { singular: "Collection", plural: "Collections" },
    item: { singular: "Document", plural: "Documents" },
    attribute: { singular: "Field", plural: "Fields" },
    reference: { singular: "Reference", plural: "References" },
    constraint: { singular: "Validation Rule", plural: "Validation Rules" },
    schema: { singular: "Database", plural: "Databases" },
    itemCount: "Document Count",
    attributeList: "Fields",
    referenceList: "References",
    actions: {
      query: "Find",
      insert: "Insert",
      update: "Update",
      delete: "Remove",
    },
    help: {
      containerDescription:
        "Collections store flexible documents with schema-less or schema-validated structures.",
      attributeDescription:
        "Fields are the data paths within documents. They can have multiple types and varying presence across documents.",
      referenceDescription:
        "References link documents across collections through ObjectIds, DBRefs, or embedded documents.",
      noContainers: "No collections found",
      noAttributes: "No fields found",
      noReferences: "No relationships detected",
    },
  },

  keyvalue: {
    container: { singular: "Bucket", plural: "Buckets" },
    item: { singular: "Entry", plural: "Entries" },
    attribute: { singular: "Key", plural: "Keys" },
    reference: { singular: "Link", plural: "Links" },
    constraint: { singular: "Rule", plural: "Rules" },
    schema: { singular: "Namespace", plural: "Namespaces" },
    itemCount: "Entry Count",
    attributeList: "Keys",
    referenceList: "Links",
    actions: {
      query: "Get",
      insert: "Set",
      update: "Set",
      delete: "Delete",
    },
    help: {
      containerDescription:
        "Buckets organize key-value pairs with fast lookup capabilities.",
      attributeDescription:
        "Keys are the identifiers used to store and retrieve values.",
      referenceDescription:
        "Links connect related entries across different buckets.",
      noContainers: "No buckets found",
      noAttributes: "No keys found",
      noReferences: "No links found",
    },
  },

  graph: {
    container: { singular: "Graph", plural: "Graphs" },
    item: { singular: "Node", plural: "Nodes" },
    attribute: { singular: "Property", plural: "Properties" },
    reference: { singular: "Edge", plural: "Edges" },
    constraint: { singular: "Constraint", plural: "Constraints" },
    schema: { singular: "Graph", plural: "Graphs" },
    itemCount: "Node Count",
    attributeList: "Properties",
    referenceList: "Edges",
    actions: {
      query: "Match",
      insert: "Create",
      update: "Set",
      delete: "Delete",
    },
    help: {
      containerDescription:
        "Graphs store interconnected nodes and relationships for complex data patterns.",
      attributeDescription:
        "Properties are key-value pairs attached to nodes and edges.",
      referenceDescription:
        "Edges define relationships between nodes with optional properties.",
      noContainers: "No graphs found",
      noAttributes: "No properties found",
      noReferences: "No edges found",
    },
  },

  search: {
    container: { singular: "Index", plural: "Indexes" },
    item: { singular: "Document", plural: "Documents" },
    attribute: { singular: "Field", plural: "Fields" },
    reference: { singular: "Alias", plural: "Aliases" },
    constraint: { singular: "Mapping Rule", plural: "Mapping Rules" },
    schema: { singular: "Index", plural: "Indexes" },
    itemCount: "Document Count",
    attributeList: "Fields",
    referenceList: "Aliases",
    actions: {
      query: "Search",
      insert: "Index",
      update: "Update",
      delete: "Delete",
    },
    help: {
      containerDescription:
        "Indexes store searchable documents optimized for full-text and analytical queries.",
      attributeDescription:
        "Fields are indexed attributes with specific analyzers and mappings.",
      referenceDescription:
        "Aliases provide alternative names and routing for indexes.",
      noContainers: "No indexes found",
      noAttributes: "No fields found",
      noReferences: "No aliases found",
    },
  },

  timeseries: {
    container: { singular: "Measurement", plural: "Measurements" },
    item: { singular: "Point", plural: "Points" },
    attribute: { singular: "Field", plural: "Fields" },
    reference: { singular: "Tag", plural: "Tags" },
    constraint: { singular: "Retention Policy", plural: "Retention Policies" },
    schema: { singular: "Database", plural: "Databases" },
    itemCount: "Point Count",
    attributeList: "Fields",
    referenceList: "Tags",
    actions: {
      query: "Select",
      insert: "Write",
      update: "Write",
      delete: "Delete",
    },
    help: {
      containerDescription:
        "Measurements store time-series data points with timestamps and tags.",
      attributeDescription:
        "Fields are numeric values recorded at each timestamp.",
      referenceDescription:
        "Tags are indexed metadata for efficient filtering and grouping.",
      noContainers: "No measurements found",
      noAttributes: "No fields found",
      noReferences: "No tags found",
    },
  },
};

/**
 * Get terminology for a specific paradigm with fallback to relational.
 */
export function getTerminology(
  paradigm: DatabaseParadigm | undefined
): Terminology {
  return TERMINOLOGY[paradigm ?? "relational"];
}

/**
 * Get a specific term for a paradigm.
 */
export function getTerm(
  paradigm: DatabaseParadigm | undefined,
  key: keyof Omit<Terminology, "actions" | "help">,
  plural = false
): string {
  const terminology = getTerminology(paradigm);
  const value = terminology[key];

  if (typeof value === "string") {
    return value;
  }

  // It's a TermPair
  return plural ? value.plural : value.singular;
}
