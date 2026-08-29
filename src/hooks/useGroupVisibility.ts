import { Adjustments } from '../utils/adjustments';

/**
 * Switching individual groups of adjustments on and off within a panel.
 *
 * Groups are stored under a dotted name so the backend can tell that a panel
 * outranks the groups inside it. Nothing is written until a group is actually
 * switched off, so an untouched image gains no keys.
 */
type AdjustmentsUpdater = (updater: (previous: Adjustments) => Adjustments) => void;

export function useGroupVisibility(panel: string, adjustments: Adjustments, setAdjustments: AdjustmentsUpdater) {
  const visibility = adjustments?.sectionVisibility || {};

  const isVisible = (group: string) => visibility[`${panel}.${group}`] !== false;

  const toggle = (group: string) =>
    setAdjustments((prev: Adjustments) => ({
      ...prev,
      sectionVisibility: {
        ...(prev.sectionVisibility || {}),
        [`${panel}.${group}`]: !isVisible(group),
      },
    }));

  return { isVisible, toggle };
}
