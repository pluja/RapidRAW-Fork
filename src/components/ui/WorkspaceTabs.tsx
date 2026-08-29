import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import { motion } from 'framer-motion';
import clsx from 'clsx';
import { Images, SlidersHorizontal, Upload } from 'lucide-react';

import { useUIStore } from '../../store/useUIStore';
import { useEditorStore } from '../../store/useEditorStore';
import { useLibraryStore } from '../../store/useLibraryStore';
import { WorkspaceId, WORKSPACE_IDS } from './AppProperties';

const WORKSPACE_ICONS: Record<WorkspaceId, typeof Images> = {
  library: Images,
  develop: SlidersHorizontal,
  export: Upload,
};

interface WorkspaceTabsProps {
  onImageSelect: (path: string, openInEditor?: boolean) => void;
}

export default function WorkspaceTabs({ onImageSelect }: WorkspaceTabsProps) {
  const { t } = useTranslation();

  const { activeView, activeWorkspace, isInstantTransition, setWorkspace } = useUIStore(
    useShallow((state) => ({
      activeView: state.activeView,
      activeWorkspace: state.activeWorkspace,
      isInstantTransition: state.isInstantTransition,
      setWorkspace: state.setWorkspace,
    })),
  );

  const selectedImagePath = useEditorStore((state) => state.selectedImage?.path ?? null);
  const libraryActivePath = useLibraryStore((state) => state.libraryActivePath);

  const developTarget = libraryActivePath ?? selectedImagePath;

  const handleSelect = useCallback(
    (id: WorkspaceId) => {
      if (id === 'develop') {
        if (activeWorkspace === 'develop' && activeView === 'editor') return;
        // Opening the photo routes through the editor view, which enters develop on its own,
        // and is a no-op when that photo is already loaded.
        if (developTarget) onImageSelect(developTarget, true);
        return;
      }

      if (activeWorkspace === 'develop' && selectedImagePath) {
        useLibraryStore.getState().setLibrary({ libraryActivePath: selectedImagePath });
      }
      setWorkspace(id);
    },
    [activeView, activeWorkspace, developTarget, selectedImagePath, setWorkspace, onImageSelect],
  );

  return (
    <div className="flex items-center gap-1 rounded-lg p-0.5">
      {WORKSPACE_IDS.map((id) => {
        const Icon = WORKSPACE_ICONS[id];
        const isActive = activeWorkspace === id;
        const isDisabled = id === 'develop' && !developTarget;

        return (
          <button
            key={id}
            disabled={isDisabled}
            onClick={() => handleSelect(id)}
            data-tooltip={t(`ui.workspaces.tooltips.${id}`)}
            className={clsx(
              'relative flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition-colors duration-200',
              isDisabled && 'opacity-40 cursor-not-allowed',
              isActive
                ? 'text-text-primary'
                : !isDisabled && 'text-text-secondary hover:bg-surface hover:text-text-primary',
            )}
          >
            {isActive && (
              <motion.div
                layoutId="active-workspace-indicator"
                className="absolute inset-0 rounded-md bg-surface"
                transition={isInstantTransition ? { duration: 0 } : { type: 'spring', bounce: 0.2, duration: 0.4 }}
              />
            )}
            <Icon size={14} className="relative z-10 pointer-events-none" />
            <span className="relative z-10 pointer-events-none">{t(`ui.workspaces.${id}`)}</span>
          </button>
        );
      })}
    </div>
  );
}
