import type { ParseKeys, TFunction } from 'i18next';

/**
 * Translates a key assembled at runtime.
 *
 * i18next derives its key type from the locale files, so a key built from a
 * template literal is never in that union however valid it is, and t() then
 * returns unknown rather than a string. Widening once here keeps that in a
 * single place with a reason attached, instead of an `as any` at each call
 * site.
 *
 * The key still has to exist: `npm run i18n:check` is what catches one that
 * does not.
 */
export const tDynamic = (t: TFunction, key: string): string => t(key as ParseKeys) as string;
