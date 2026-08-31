/**
 * Shared audio helpers for locating assets with multiple extensions.
 *
 * This keeps canonical PCM discovery logic centralized so the game,
 * exporters, and dataset builders all agree on available audio formats.
 */

import * as path from 'path';
import * as fs from 'fs';

export const AUDIO_EXTENSIONS = ['.pcm'];
export const MUSIC_AUDIO_EXTENSIONS = ['.pcm'];
export const PCM_AUDIO_EXTENSIONS = ['.pcm'];

const _EXTENSION_PRIORITIES = Object.fromEntries(
  AUDIO_EXTENSIONS.map((ext, idx) => [ext, idx])
);
const _MUSIC_EXTENSION_PRIORITIES = Object.fromEntries(
  MUSIC_AUDIO_EXTENSIONS.map((ext, idx) => [ext, idx])
);

function _find_with_extensions(
  directory: string,
  basename: string,
  extensions: string[]
): string | null {
  for (const ext of extensions) {
    const candidate = path.join(directory, `${basename}${ext}`);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return null;
}

export function find_audio_file(directory: string, basename: string): string | null {
  return _find_with_extensions(directory, basename, AUDIO_EXTENSIONS);
}

export function find_music_file(directory: string, basename: string): string | null {
  return _find_with_extensions(directory, basename, MUSIC_AUDIO_EXTENSIONS);
}

export function find_pcm_audio_file(directory: string, basename: string): string | null {
  return _find_with_extensions(directory, basename, PCM_AUDIO_EXTENSIONS);
}

function _select_preferred_files(
  paths: string[],
  allowed_extensions: string[],
  priorities: { [key: string]: number }
): { [key: string]: string } {
  const selected: { [key: string]: string } = {};
  const allowed = new Set(allowed_extensions);
  const fallback_priority = allowed_extensions.length;

  for (const p of paths) {
    try {
      const stat = fs.statSync(p);
      if (!stat.isFile()) continue;
    } catch (e) {
      continue; // File doesn't exist or other error
    }

    const ext = path.extname(p).toLowerCase();
    if (ext === '' || !allowed.has(ext)) {
      continue;
    }

    const stem = path.basename(p, ext);
    const priority = priorities[ext] ?? fallback_priority;
    const existing = selected[stem];

    if (!existing) {
      selected[stem] = p;
      continue;
    }

    const existing_priority = priorities[path.extname(existing).toLowerCase()] ?? fallback_priority;
    if (priority < existing_priority) {
      selected[stem] = p;
    }
  }
  return selected;
}

export function select_preferred_audio_files(paths: string[]): { [key: string]: string } {
  return _select_preferred_files(paths, AUDIO_EXTENSIONS, _EXTENSION_PRIORITIES);
}

export function select_preferred_music_files(paths: string[]): { [key: string]: string } {
  return _select_preferred_files(paths, MUSIC_AUDIO_EXTENSIONS, _MUSIC_EXTENSION_PRIORITIES);
}
