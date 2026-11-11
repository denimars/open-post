/**
 * The only module that calls `invoke()`.
 * Everything else uses these typed wrappers.
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  Collection,
  CollectionImportReport,
  Environment,
  EnvironmentImportReport,
  ExportFile,
  HistoryEntry,
  HttpRequest,
  HttpResponse,
  SavedExport,
  SavedRequest,
} from "@/types";
import { toAppError } from "@/types";

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

/**
 * A storage problem found while the app was starting (for example a database
 * file left unusable by a crash), or `null` on a normal start.
 */
export function startupWarning(): Promise<string | null> {
  return call<string | null>("startup_warning");
}

export function sendRequest(
  request: HttpRequest,
  environmentId?: string | null,
): Promise<HttpResponse> {
  return call<HttpResponse>("send_request", {
    request,
    environmentId: environmentId ?? null,
  });
}

export function cancelRequest(requestId: string): Promise<boolean> {
  return call<boolean>("cancel_request", { requestId });
}

export function listHistory(limit = 100): Promise<HistoryEntry[]> {
  return call<HistoryEntry[]>("list_history", { limit });
}

export function clearHistory(): Promise<void> {
  return call<void>("clear_history");
}

export function listCollections(): Promise<Collection[]> {
  return call<Collection[]>("list_collections");
}

export function createCollection(name: string): Promise<Collection> {
  return call<Collection>("create_collection", { name });
}

export function renameCollection(id: string, name: string): Promise<void> {
  return call<void>("rename_collection", { id, name });
}

export function deleteCollection(id: string): Promise<void> {
  return call<void>("delete_collection", { id });
}

/**
 * Saves a request into a collection. `savedRequestId` updates that existing
 * entry; omitting it always creates a new one.
 */
export function saveRequest(
  collectionId: string,
  name: string,
  request: HttpRequest,
  savedRequestId: string | null = null,
): Promise<SavedRequest> {
  return call<SavedRequest>("save_request", {
    collectionId,
    name,
    request,
    savedRequestId,
  });
}

export function deleteRequest(id: string): Promise<void> {
  return call<void>("delete_request", { id });
}

export function listEnvironments(): Promise<Environment[]> {
  return call<Environment[]>("list_environments");
}

export function saveEnvironment(environment: Environment): Promise<Environment> {
  return call<Environment>("save_environment", { environment });
}

export function deleteEnvironment(id: string): Promise<void> {
  return call<void>("delete_environment", { id });
}

export function setActiveEnvironment(id: string | null): Promise<void> {
  return call<void>("set_active_environment", { id });
}

export function getActiveEnvironment(): Promise<string | null> {
  return call<string | null>("get_active_environment");
}

// ----------------------------------------------- Postman import / export

/** Returns the collection as Postman v2.1 JSON, without touching the disk. */
export function exportCollection(collectionId: string): Promise<ExportFile> {
  return call<ExportFile>("export_collection", { collectionId });
}

/** Writes the collection to `path`, or to the download directory when omitted. */
export function exportCollectionToFile(
  collectionId: string,
  path: string | null = null,
): Promise<SavedExport> {
  return call<SavedExport>("export_collection_to_file", { collectionId, path });
}

export function importCollection(json: string): Promise<CollectionImportReport> {
  return call<CollectionImportReport>("import_collection", { json });
}

export function importCollectionFromFile(path: string): Promise<CollectionImportReport> {
  return call<CollectionImportReport>("import_collection_from_file", { path });
}

export function exportEnvironment(environmentId: string): Promise<ExportFile> {
  return call<ExportFile>("export_environment", { environmentId });
}

export function exportEnvironmentToFile(
  environmentId: string,
  path: string | null = null,
): Promise<SavedExport> {
  return call<SavedExport>("export_environment_to_file", { environmentId, path });
}

export function importEnvironment(json: string): Promise<EnvironmentImportReport> {
  return call<EnvironmentImportReport>("import_environment", { json });
}

export function importEnvironmentFromFile(path: string): Promise<EnvironmentImportReport> {
  return call<EnvironmentImportReport>("import_environment_from_file", { path });
}
