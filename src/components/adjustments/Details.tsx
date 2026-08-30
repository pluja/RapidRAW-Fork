import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import GroupVisibilityToggle from '../ui/GroupVisibilityToggle';
import { useGroupVisibility } from '../../hooks/useGroupVisibility';
import Slider from '../ui/Slider';
import { Adjustments, DetailsAdjustment } from '../../utils/adjustments';
import { AppSettings } from '../ui/AppProperties';
import Text from '../ui/Text';
import { TextVariants } from '../../types/typography';

interface DetailsPanelProps {
  adjustments: Adjustments;
  setAdjustments(adjustments: Partial<Adjustments>): any;
  appSettings: AppSettings | null;
  isForMask?: boolean;
  onDragStateChange?: (isDragging: boolean) => void;
}

export default function DetailsPanel({
  adjustments,
  setAdjustments,
  appSettings,
  isForMask = false,
  onDragStateChange,
}: DetailsPanelProps) {
  const groups = useGroupVisibility('details', adjustments, setAdjustments);

  // The mask shows while the slider is being dragged, which is when it is worth
  // seeing. Lightroom puts this behind alt, but KDE claims alt-drag for moving
  // windows, and a modifier earns nothing here anyway.
  const [isMaskingDrag, setIsMaskingDrag] = useState(false);

  useEffect(() => {
    setAdjustments((prev: Adjustments) =>
      (prev.sharpenMaskPreview ?? false) === isMaskingDrag ? prev : { ...prev, sharpenMaskPreview: isMaskingDrag },
    );
  }, [isMaskingDrag, setAdjustments]);
  const { t } = useTranslation();

  // Sliders hand back a number, which parseInt only accepted because JavaScript
  // stringified it first, truncating anything below a whole step on the way.
  const handleAdjustmentChange = (key: string, value: number | string) => {
    const numericValue = Number(value);
    if (Number.isNaN(numericValue)) {
      return;
    }
    setAdjustments((prev: Partial<Adjustments>) => ({ ...prev, [key]: numericValue }));
  };

  const adjustmentVisibility = appSettings?.adjustmentVisibility || {};

  return (
    <div className="space-y-2">
      {adjustmentVisibility.sharpening !== false && (
        <div className="p-2 bg-bg-tertiary rounded-md">
          <div className="group/group flex justify-between items-center mb-2">
            <Text variant={TextVariants.heading}>{t('adjustments.details.sharpening')}</Text>
            <GroupVisibilityToggle
              isVisible={groups.isVisible('sharpening')}
              onToggle={() => groups.toggle('sharpening')}
            />
          </div>
          <Slider
            label={t('adjustments.details.sharpness')}
            max={100}
            min={-100}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.Sharpness, e.target.value)}
            step={1}
            value={adjustments.sharpness}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('adjustments.details.threshold')}
            max={80}
            min={0}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.SharpnessThreshold, e.target.value)}
            step={1}
            value={adjustments.sharpnessThreshold ?? 15}
            onDragStateChange={onDragStateChange}
            defaultValue={15}
            fillOrigin="min"
          />
          <div data-tooltip={t('adjustments.details.maskingTooltip')}>
            <Slider
              label={t('adjustments.details.masking')}
              max={100}
              min={0}
              onChange={(e: { target: { value: number } }) =>
                handleAdjustmentChange(DetailsAdjustment.SharpenMasking, e.target.value)
              }
              step={1}
              value={adjustments.sharpenMasking ?? 0}
              onDragStateChange={(dragging: boolean) => {
                setIsMaskingDrag(dragging);
                onDragStateChange?.(dragging);
              }}
              defaultValue={0}
              fillOrigin="min"
            />
          </div>
        </div>
      )}

      {adjustmentVisibility.presence !== false && (
        <div className="p-2 bg-bg-tertiary rounded-md">
          <div className="group/group flex justify-between items-center mb-2">
            <Text variant={TextVariants.heading}>{t('adjustments.details.presence')}</Text>
            <GroupVisibilityToggle
              isVisible={groups.isVisible('presence')}
              onToggle={() => groups.toggle('presence')}
            />
          </div>
          <Slider
            label={t('adjustments.details.clarity')}
            max={100}
            min={-100}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.Clarity, e.target.value)}
            step={1}
            value={adjustments.clarity}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('adjustments.details.dehaze')}
            max={100}
            min={-100}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.Dehaze, e.target.value)}
            step={1}
            value={adjustments.dehaze}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('adjustments.details.structure')}
            max={100}
            min={-100}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.Structure, e.target.value)}
            step={1}
            value={adjustments.structure}
            onDragStateChange={onDragStateChange}
          />
          {!isForMask && (
            <Slider
              label={t('adjustments.details.centre')}
              max={100}
              min={-100}
              onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.Centré, e.target.value)}
              step={1}
              value={adjustments.centré}
              onDragStateChange={onDragStateChange}
            />
          )}
        </div>
      )}

      {adjustmentVisibility.noiseReduction !== false && (
        <div className="p-2 bg-bg-tertiary rounded-md">
          <div className="group/group flex justify-between items-center mb-2">
            <Text variant={TextVariants.heading}>{t('adjustments.details.noiseReduction')}</Text>
            <GroupVisibilityToggle
              isVisible={groups.isVisible('noiseReduction')}
              onToggle={() => groups.toggle('noiseReduction')}
            />
          </div>
          <Slider
            label={t('adjustments.details.luminance')}
            max={100}
            min={isForMask ? -100 : 0}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.LumaNoiseReduction, e.target.value)}
            step={1}
            value={adjustments.lumaNoiseReduction}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('adjustments.details.color')}
            max={100}
            min={isForMask ? -100 : 0}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.ColorNoiseReduction, e.target.value)}
            step={1}
            value={adjustments.colorNoiseReduction}
            onDragStateChange={onDragStateChange}
          />
        </div>
      )}

      {!isForMask && adjustmentVisibility.chromaticAberration !== false && (
        <div className="p-2 bg-bg-tertiary rounded-md">
          <div className="group/group flex justify-between items-center mb-2">
            <Text variant={TextVariants.heading}>{t('adjustments.details.chromaticAberration')}</Text>
            <GroupVisibilityToggle
              isVisible={groups.isVisible('chromaticAberration')}
              onToggle={() => groups.toggle('chromaticAberration')}
            />
          </div>
          <Slider
            label={t('adjustments.details.redCyan')}
            max={100}
            min={-100}
            onChange={(e: any) => handleAdjustmentChange(DetailsAdjustment.ChromaticAberrationRedCyan, e.target.value)}
            step={1}
            value={adjustments.chromaticAberrationRedCyan}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('adjustments.details.blueYellow')}
            max={100}
            min={-100}
            onChange={(e: any) =>
              handleAdjustmentChange(DetailsAdjustment.ChromaticAberrationBlueYellow, e.target.value)
            }
            step={1}
            value={adjustments.chromaticAberrationBlueYellow}
            onDragStateChange={onDragStateChange}
          />
        </div>
      )}
    </div>
  );
}
