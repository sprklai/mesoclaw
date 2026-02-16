/**
 * Path utilities for file system operations.
 *
 * This module provides cross-platform path manipulation functions
 * that work with both Unix-style and Windows-style paths.
 */

/**
 * Extract the file name from a path.
 *
 * Works with both Unix-style forward slashes and Windows-style backslashes.
 *
 * @param path - The full file path
 * @returns The file name including extension, or the original path if no separator found
 *
 * @example
 * ```ts
 * getFileName("/home/user/database.db") // "database.db"
 * getFileName("C:\\Users\\data.sqlite") // "data.sqlite"
 * getFileName("file.txt") // "file.txt"
 * ```
 */
export function getFileName(path: string): string {
  if (!path) return "";

  // Handle both forward and backslashes
  const normalizedPath = path.replace(/\\/g, "/");
  const lastSeparator = normalizedPath.lastIndexOf("/");

  if (lastSeparator === -1) {
    return path;
  }

  return normalizedPath.slice(lastSeparator + 1);
}

/**
 * Extract the file extension from a path or filename.
 *
 * @param path - The file path or filename
 * @returns The extension without the dot, or empty string if no extension
 *
 * @example
 * ```ts
 * getFileExtension("database.db") // "db"
 * getFileExtension("/path/to/file.sqlite") // "sqlite"
 * getFileExtension("noextension") // ""
 * getFileExtension(".gitignore") // "gitignore"
 * ```
 */
export function getFileExtension(path: string): string {
  if (!path) return "";

  const fileName = getFileName(path);
  const lastDot = fileName.lastIndexOf(".");

  // No extension or hidden file without extension (like ".gitignore" is "gitignore")
  if (lastDot === -1) {
    return "";
  }

  // Hidden file like ".gitignore" - the extension is everything after the dot
  if (lastDot === 0) {
    return fileName.slice(1);
  }

  return fileName.slice(lastDot + 1);
}

/**
 * Extract the base name (file name without extension) from a path.
 *
 * @param path - The file path or filename
 * @returns The base name without extension
 *
 * @example
 * ```ts
 * getBaseName("database.db") // "database"
 * getBaseName("/path/to/file.sqlite") // "file"
 * getBaseName("noextension") // "noextension"
 * ```
 */
export function getBaseName(path: string): string {
  if (!path) return "";

  const fileName = getFileName(path);
  const lastDot = fileName.lastIndexOf(".");

  if (lastDot <= 0) {
    // No extension or hidden file
    return fileName;
  }

  return fileName.slice(0, lastDot);
}

/**
 * Extract the directory path from a full file path.
 *
 * @param path - The full file path
 * @returns The directory path, or empty string if no directory
 *
 * @example
 * ```ts
 * getDirectoryPath("/home/user/database.db") // "/home/user"
 * getDirectoryPath("C:\\Users\\data.sqlite") // "C:/Users"
 * getDirectoryPath("file.txt") // ""
 * ```
 */
export function getDirectoryPath(path: string): string {
  if (!path) return "";

  // Handle both forward and backslashes
  const normalizedPath = path.replace(/\\/g, "/");
  const lastSeparator = normalizedPath.lastIndexOf("/");

  if (lastSeparator === -1) {
    return "";
  }

  return normalizedPath.slice(0, lastSeparator);
}

/**
 * Remove common database file extensions from a filename.
 *
 * Useful for generating workspace names from database file paths.
 *
 * @param fileName - The filename to clean
 * @returns The filename without common database extensions
 *
 * @example
 * ```ts
 * stripDatabaseExtension("mydata.db") // "mydata"
 * stripDatabaseExtension("users.sqlite") // "users"
 * stripDatabaseExtension("database.sqlite3") // "database"
 * ```
 */
export function stripDatabaseExtension(fileName: string): string {
  const extensions = [".db", ".sqlite", ".sqlite3", ".db3"];

  let result = fileName;
  for (const ext of extensions) {
    if (result.toLowerCase().endsWith(ext)) {
      result = result.slice(0, -ext.length);
      break;
    }
  }

  return result;
}
