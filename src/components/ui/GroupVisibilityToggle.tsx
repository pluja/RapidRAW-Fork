import { Eye, EyeOff } from 'lucide-react';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';

interface GroupVisibilityToggleProps {
  isVisible: boolean;
  onToggle: () => void;
}

/**
 * Switches one group of adjustments off, matching the eye that switches off a
 * whole panel. It stays hidden until the heading is hovered so a panel of six
 * groups does not read as six buttons, and stays put once a group is off so the
 * way back is always visible.
 */
export default function GroupVisibilityToggle({ isVisible, onToggle }: GroupVisibilityToggleProps) {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      aria-pressed={!isVisible}
      onClick={onToggle}
      data-tooltip={isVisible ? t('ui.collapsibleSection.disableSection') : t('ui.collapsibleSection.enableSection')}
      className={clsx(
        'p-1 rounded-full text-text-secondary hover:bg-bg-primary transition-opacity duration-200',
        isVisible ? 'opacity-0 group-hover/group:opacity-100 focus-visible:opacity-100' : 'opacity-100',
      )}
    >
      {isVisible ? <Eye size={16} /> : <EyeOff size={16} />}
    </button>
  );
}
