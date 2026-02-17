/**
 * Contextual help content for different sections of the application.
 *
 * This module provides paradigm-aware help content that adapts terminology
 * based on the database type (relational vs document vs keyvalue, etc.).
 */

import {
  type DatabaseParadigm,
  type Terminology,
  getTerminology,
} from "./terminology";
import { APP_IDENTITY } from "../config/app-identity";

/**
 * Help content structure for a specific section.
 */
interface HelpSection {
  title: string;
  content: string;
}

/**
 * Complete help content structure for the application.
 */
export interface HelpContent {
  /** Help for attributes (columns/fields) */
  attributes: HelpSection;
  /** Help for relationships (foreign keys/references) */
  relationships: HelpSection;
  /** Help for statistics */
  statistics: HelpSection;
  /** Help for data viewing */
  data: HelpSection;
  /** Help for AI explanations */
  containerExplanation: HelpSection;
  /** Onboarding help content */
  onboarding: {
    welcome: HelpSection;
    schemaExplorer: HelpSection;
    firstConnection: HelpSection;
  };
}

/**
 * Generate paradigm-aware help content.
 *
 * @param paradigm - The database paradigm (relational, document, etc.)
 * @returns Help content with appropriate terminology
 */
export function getHelpContent(paradigm?: DatabaseParadigm): HelpContent {
  const t = getTerminology(paradigm);

  return {
    attributes: {
      title: `Understanding ${t.attributeList}`,
      content: generateAttributeHelp(t),
    },
    relationships: {
      title: `Understanding ${t.referenceList}`,
      content: generateRelationshipHelp(t),
    },
    statistics: {
      title: "Understanding Statistics",
      content: generateStatisticsHelp(t),
    },
    data: {
      title: `Understanding ${t.container.singular} Data`,
      content: generateDataHelp(t),
    },
    containerExplanation: {
      title: `AI-Generated ${t.container.singular} Explanation`,
      content: generateExplanationHelp(t),
    },
    onboarding: {
      welcome: {
        title: `Welcome to ${APP_IDENTITY.productName}!`,
        content: generateWelcomeHelp(t),
      },
      schemaExplorer: {
        title: "Schema Explorer Guide",
        content: generateSchemaExplorerHelp(t),
      },
      firstConnection: {
        title: "Your First Connection",
        content: generateFirstConnectionHelp(),
      },
    },
  };
}

function generateAttributeHelp(t: Terminology): string {
  const isRelational = t.container.singular === "Table";

  if (isRelational) {
    return `
### Understanding ${t.attributeList}

**${t.attributeList}** define the structure of your ${t.container.singular.toLowerCase()} data. Each ${t.attribute.singular.toLowerCase()} has:

- **Name**: The identifier used in queries
- **Type**: The data type (TEXT, INTEGER, REAL, BLOB, etc.)
- **Constraints**: Rules like NOT NULL, UNIQUE, PRIMARY KEY
- **Default Values**: Automatic values when not specified

**Key Points:**
- Primary keys uniquely identify each ${t.item.singular.toLowerCase()}
- ${t.referenceList} create relationships to other ${t.container.plural.toLowerCase()}
- Nullable ${t.attribute.plural.toLowerCase()} can contain NULL values
- Indexed ${t.attribute.plural.toLowerCase()} improve query performance
    `;
  }

  // Document database
  return `
### Understanding ${t.attributeList}

**${t.attributeList}** are the data paths within ${t.item.plural.toLowerCase()}. Each ${t.attribute.singular.toLowerCase()} has:

- **Name**: The path to the data (supports dot notation for nested fields)
- **Types**: Observed BSON types across ${t.item.plural.toLowerCase()}
- **Presence**: Percentage of ${t.item.plural.toLowerCase()} containing this ${t.attribute.singular.toLowerCase()}
- **Properties**: Whether indexed, array, embedded, or reference

**Key Points:**
- ${t.attributeList} with <100% presence are optional
- Nested ${t.attribute.plural.toLowerCase()} appear with dot notation (e.g., address.city)
- Reference ${t.attribute.plural.toLowerCase()} link to other ${t.container.plural.toLowerCase()}
- Array ${t.attribute.plural.toLowerCase()} can contain multiple values
  `;
}

function generateRelationshipHelp(t: Terminology): string {
  const isRelational = t.container.singular === "Table";

  if (isRelational) {
    return `
### Understanding ${t.referenceList}

**${t.referenceList}** connect ${t.container.plural.toLowerCase()} through foreign key constraints, enabling:

- **Outgoing**: This ${t.container.singular.toLowerCase()} references another ${t.container.singular.toLowerCase()}
- **Incoming**: Another ${t.container.singular.toLowerCase()} references this ${t.container.singular.toLowerCase()}

**Relationship Types:**
- **One-to-Many (1:N)**: One ${t.item.singular.toLowerCase()} relates to multiple ${t.item.plural.toLowerCase()}
- **Many-to-One (N:1)**: Multiple ${t.item.plural.toLowerCase()} relate to one ${t.item.singular.toLowerCase()}
- **One-to-One (1:1)**: Single ${t.item.singular.toLowerCase()} to single ${t.item.singular.toLowerCase()}

**Navigation:**
Click on a related ${t.container.singular.toLowerCase()} to explore its structure and connections.
    `;
  }

  // Document database
  return `
### Understanding ${t.referenceList}

**${t.referenceList}** connect ${t.container.plural.toLowerCase()} through different mechanisms:

- **Manual Reference**: ObjectId stored in a ${t.attribute.singular.toLowerCase()}
- **DBRef**: MongoDB's structured reference format
- **Embedded**: ${t.item.plural} nested within other ${t.item.plural.toLowerCase()}
- **Array Reference**: Multiple ObjectIds in an array

**Relationship Types:**
- **One-to-Many**: One ${t.item.singular.toLowerCase()} embeds/references many
- **Many-to-One**: Many ${t.item.plural.toLowerCase()} reference one
- **One-to-One**: Single ${t.item.singular.toLowerCase()} embedded or referenced

**Navigation:**
Click on a related ${t.container.singular.toLowerCase()} to explore its structure.
  `;
}

function generateStatisticsHelp(t: Terminology): string {
  return `
### Understanding Statistics

**${t.container.singular} Statistics** provide insights into your data:

**Performance Metrics:**
- **${t.itemCount}**: Total number of ${t.item.plural.toLowerCase()}
- **Size**: Storage space used (KB/MB)
- **Last Modified**: Most recent change timestamp

**Schema Metrics:**
- **${t.attributeList}**: Number of ${t.attribute.plural.toLowerCase()} in the ${t.container.singular.toLowerCase()}
- **Indexes**: Performance optimization structures
- **${t.referenceList}**: Total connections to other ${t.container.plural.toLowerCase()}

Use these metrics to monitor data growth and optimize queries.
  `;
}

function generateDataHelp(t: Terminology): string {
  return `
### Understanding ${t.container.singular} Data

**${t.container.singular} Data** shows the actual content stored:

**Features:**
- **Pagination**: Browse 50 ${t.item.plural.toLowerCase()} at a time
- **Navigation**: Use Previous/Next buttons
- **NULL Values**: Displayed as empty/light gray

**Data Types:**
- Text shows as strings
- Numbers display as values
- Booleans show as true/false
- NULL values are clearly marked

**Performance:**
- Large ${t.container.plural.toLowerCase()} load data in chunks
- Scroll to explore all ${t.item.plural.toLowerCase()}
- Sort by ${t.attribute.singular.toLowerCase()} (coming soon)
  `;
}

function generateExplanationHelp(t: Terminology): string {
  return `
### AI-Generated ${t.container.singular} Explanation

This explanation is generated using AI to help you understand:

- **Purpose**: What this ${t.container.singular.toLowerCase()} is designed to store
- **Structure**: How the ${t.attribute.plural.toLowerCase()} work together
- **Key Observations**: Important patterns and constraints
- **Confidence**: How certain the AI is about the analysis

**Note:** AI explanations are cached for performance. The analysis is based on:
- ${t.attribute.singular} names and types
- Constraints and ${t.reference.plural.toLowerCase()}
- Common database patterns
  `;
}

function generateWelcomeHelp(t: Terminology): string {
  return `
# Welcome to ${APP_IDENTITY.productName}!

**${APP_IDENTITY.productName}** is your intelligent database exploration companion.

## Getting Started

1. **Connect**: Add your database connection
2. **Explore**: Browse ${t.container.plural.toLowerCase()} and ${t.reference.plural.toLowerCase()}
3. **Understand**: Get AI-powered explanations
4. **${t.actions.query}**: Run queries with assistance

### Key Features

- **Schema Explorer**: Visual ${t.container.singular.toLowerCase()} ${t.reference.plural.toLowerCase()}
- **AI Explanations**: Understand ${t.container.singular.toLowerCase()} purposes
- **Statistics**: Monitor data metrics
- **${t.referenceList}**: Navigate connections

**Tip**: Click "Explain ${t.container.singular}" on any ${t.container.singular.toLowerCase()} to get AI-powered insights!
  `;
}

function generateSchemaExplorerHelp(t: Terminology): string {
  return `
### Schema Explorer Guide

The **Schema Explorer** helps you navigate your database structure:

**Left Panel**: Tree view of all ${t.container.plural.toLowerCase()}
- Click to select a ${t.container.singular.toLowerCase()}
- Expand to see ${t.attribute.plural.toLowerCase()}

**Right Panel**: Detailed ${t.container.singular.toLowerCase()} information
- **${t.attributeList} Tab**: ${t.attribute.singular} definitions
- **${t.referenceList} Tab**: ${t.reference.singular} connections
- **Statistics Tab**: Data metrics

**Pro Tips:**
- Use the search to find ${t.container.plural.toLowerCase()} quickly
- Click related ${t.container.plural.toLowerCase()} to navigate
- Request AI explanations for complex ${t.container.plural.toLowerCase()}
  `;
}

function generateFirstConnectionHelp(): string {
  return `
### Your First Connection

To connect to a database:

1. Click **"New Connection"**
2. Choose your database type (SQLite, PostgreSQL, MySQL, MongoDB)
3. Enter connection details
4. Test the connection
5. Save and explore!

**Supported Databases:**
- SQLite (file-based, local)
- PostgreSQL (relational)
- MySQL (relational)
- MongoDB (document)

**Security**: Connection details are stored locally and encrypted.
  `;
}

/**
 * Legacy static help content for backwards compatibility.
 * @deprecated Use getHelpContent(paradigm) instead for paradigm-aware content.
 */
export const HELP_CONTENT = {
  columns: `
### Understanding Columns

**Columns** define the structure of your table data. Each column has:

- **Name**: The identifier used in queries
- **Type**: The data type (TEXT, INTEGER, REAL, BLOB, etc.)
- **Constraints**: Rules like NOT NULL, UNIQUE, PRIMARY KEY
- **Default Values**: Automatic values when not specified

**Key Points:**
- Primary keys uniquely identify each row
- Foreign keys create relationships to other tables
- Nullable columns can contain NULL values
- Indexed columns improve query performance
  `,

  relationships: `
### Understanding Relationships

**Relationships** connect tables through foreign keys, enabling:

- **Outgoing**: This table references another table
- **Incoming**: Another table references this table

**Relationship Types:**
- **One-to-Many**: One record relates to multiple records
- **Many-to-One**: Multiple records relate to one record
- **One-to-One**: Single record to single record

**Navigation:**
Click on a related table to explore its structure and connections.
  `,

  statistics: `
### Understanding Statistics

**Table Statistics** provide insights into your data:

**Performance Metrics:**
- **Row Count**: Total number of records
- **Size**: Storage space used (KB/MB)
- **Last Modified**: Most recent change timestamp

**Schema Metrics:**
- **Columns**: Number of fields in the table
- **Indexes**: Performance optimization structures
- **Relationships**: Total foreign key connections

Use these metrics to monitor data growth and optimize queries.
  `,

  data: `
### Understanding Table Data

**Table Data** shows the actual content stored in the table:

**Features:**
- **Pagination**: Browse 50 rows at a time
- **Navigation**: Use Previous/Next buttons
- **NULL Values**: Displayed as empty/light gray

**Data Types:**
- Text shows as strings
- Numbers display as values
- Booleans show as true/false
- NULL values are clearly marked

**Performance:**
- Large tables load data in chunks
- Scroll to explore all rows
- Sort by column (coming soon)
  `,

  tableExplanation: `
### AI-Generated Table Explanation

This explanation is generated using AI to help you understand:

- **Purpose**: What this table is designed to store
- **Structure**: How the columns work together
- **Key Observations**: Important patterns and constraints
- **Confidence**: How certain the AI is about the analysis

**Note:** AI explanations are cached for performance. The analysis is based on:
- Column names and types
- Constraints and relationships
- Common database patterns
  `,

  onboarding: {
    welcome: `
# Welcome to ${APP_IDENTITY.productName}! 👋

**${APP_IDENTITY.productName}** is your intelligent database exploration companion.

## Getting Started

1. **Connect**: Add your database connection
2. **Explore**: Browse tables and relationships
3. **Understand**: Get AI-powered explanations
4. **Query**: Run SQL queries with assistance

### Key Features

- 🔍 **Schema Explorer**: Visual table relationships
- 🤖 **AI Explanations**: Understand table purposes
- 📊 **Statistics**: Monitor data metrics
- 🔗 **Relationships**: Navigate foreign keys

**Tip**: Click "Explain Table" on any table to get AI-powered insights!
    `,

    schemaExplorer: `
### Schema Explorer Guide

The **Schema Explorer** helps you navigate your database structure:

**Left Panel**: Tree view of all tables
- Click to select a table
- Expand to see columns

**Right Panel**: Detailed table information
- **Columns Tab**: Field definitions
- **Relationships Tab**: Foreign key connections
- **Statistics Tab**: Data metrics

**Pro Tips:**
- Use the search to find tables quickly
- Click related tables to navigate
- Request AI explanations for complex tables
    `,

    firstConnection: `
### Your First Connection

To connect to a database:

1. Click **"New Connection"**
2. Choose your database type (SQLite, PostgreSQL, etc.)
3. Enter connection details
4. Test the connection
5. Save and explore!

**Supported Databases:**
- SQLite (file-based)
- PostgreSQL
- MySQL
- And more...

**Security**: Connection details are stored locally and encrypted.
    `,
  },
};
