import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { GLOBAL_KEYS } from './AppProperties';

type SliderChangeEvent =
  | React.ChangeEvent<HTMLInputElement>
  | {
      target: {
        value: number | string;
      };
    };

interface SliderProps {
  defaultValue?: number;
  disabled?: boolean;
  label: React.ReactNode;
  max: number;
  min: number;
  onChange(event: SliderChangeEvent): void;
  onDragStateChange?(state: boolean): void;
  onPointerUp?(): void;
  step: number;
  value: number;
  trackClassName?: string;
  fillOrigin?: 'min' | 'default';
  suffix?: string;
}

const DOUBLE_CLICK_THRESHOLD_MS = 150;
const FINE_ADJUSTMENT_MULTIPLIER = 0.2;

// The thumb follows the pointer, so one track width crosses the range, as it
// does in Lightroom and Capture One. Any longer travel puts the maximum past
// the screen edge, because these panels sit against it: at the 650px this used
// to ask for, a 154px track needed 325px of rightward room and had 145px.
//
// Sensitivity therefore follows panel width. Both references accept that, and
// widening the panel is the remedy each of them documents.
const THUMB_GRAB_RADIUS_PX = 10;

// Dragging the readout scrubs at a finer gearing. Lightroom carries precision
// on this second surface rather than on a modifier, and it is the method its
// documentation and tutorials point to.
const VALUE_SCRUB_MULTIPLIER = 0.25;
const VALUE_SCRUB_THRESHOLD_PX = 3;

const TOUCH_DRAG_THRESHOLD_PX = 10;
const TOUCH_THUMB_HIT_RADIUS_PX = 24;

// Shift only. Alt is reserved for the previews that show what a slider is
// acting on, which is what it does in Lightroom.
const hasFineAdjustmentModifier = (event: MouseEvent | TouchEvent | React.MouseEvent | React.TouchEvent) =>
  'shiftKey' in event && event.shiftKey;

const Slider = ({
  defaultValue = 0,
  disabled = false,
  label,
  max,
  min,
  onChange,
  onDragStateChange = () => {},
  onPointerUp,
  step = 1,
  value,
  trackClassName,
  fillOrigin = 'default',
  suffix = '',
}: SliderProps) => {
  const { t } = useTranslation();
  const [displayValue, setDisplayValue] = useState<number>(value);
  // The thumb draws from the unsnapped position so it glides, while the value
  // handed to the parent and shown in the readout still snaps to the step.
  const [smoothValue, setSmoothValue] = useState<number>(value);
  const [isDragging, setIsDragging] = useState(false);
  const animationFrameRef = useRef<number | undefined>(undefined);
  const [isEditing, setIsEditing] = useState(false);
  const [inputValue, setInputValue] = useState<string>(String(value));
  const inputRef = useRef<HTMLInputElement | null>(null);
  const rangeInputRef = useRef<HTMLInputElement | null>(null);
  const [isLabelHovered, setIsLabelHovered] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const lastUpTime = useRef(0);
  const lastPointerXRef = useRef<number>(0);
  const accumulatedValueRef = useRef<number>(0);
  const dragGeometryRef = useRef({ left: 0, width: 1, grabOffset: 0 });
  const fineAnchorRef = useRef<{ x: number; value: number } | null>(null);
  const suppressValueClickRef = useRef(false);
  const [isScrubbingValue, setIsScrubbingValue] = useState(false);
  const pendingTouchRef = useRef<{
    startX: number;
    startY: number;
    latestX: number;
    startValue: number;
  } | null>(null);
  const suppressTouchChangeRef = useRef(false);
  const isWheelActivelyChangingRef = useRef(false);
  const wheelTimeoutRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    return () => {
      if (wheelTimeoutRef.current !== undefined) {
        window.clearTimeout(wheelTimeoutRef.current);
      }
    };
  }, []);

  const isInteracting = isDragging || isScrubbingValue;
  const thumbValue = isInteracting ? smoothValue : displayValue;
  const fillPercentage = max !== min ? ((thumbValue - min) / (max - min)) * 100 : 0;
  const originPercentage = useMemo(() => {
    if (fillOrigin === 'min') {
      return 0;
    }
    return max !== min ? ((defaultValue - min) / (max - min)) * 100 : 0;
  }, [fillOrigin, defaultValue, min, max]);

  const stepStr = String(step);
  const decimalPlaces = stepStr.includes('.') ? stepStr.split('.')[1].length : 0;

  const snapToStep = useCallback(
    (val: number): number => {
      const snapped = Math.round((val - min) / step) * step + min;
      const clamped = Math.max(min, Math.min(max, snapped));
      return parseFloat(clamped.toFixed(decimalPlaces));
    },
    [min, max, step, decimalPlaces],
  );

  const onChangeRef = useRef(onChange);
  const snapToStepRef = useRef(snapToStep);
  const rangeRef = useRef({ min, max });

  onChangeRef.current = onChange;
  snapToStepRef.current = snapToStep;
  rangeRef.current = { min, max };

  const onDragStateChangeRef = useRef(onDragStateChange);
  onDragStateChangeRef.current = onDragStateChange;

  useEffect(() => {
    onDragStateChangeRef.current(isInteracting);
  }, [isInteracting]);

  useEffect(() => {
    if (!disabled) return;

    pendingTouchRef.current = null;
    suppressTouchChangeRef.current = false;
    isWheelActivelyChangingRef.current = false;

    if (wheelTimeoutRef.current !== undefined) {
      window.clearTimeout(wheelTimeoutRef.current);
      wheelTimeoutRef.current = undefined;
    }
    if (animationFrameRef.current !== undefined) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = undefined;
    }

    setIsDragging(false);
    setIsEditing(false);
    setIsLabelHovered(false);
    setDisplayValue(value);
    setSmoothValue(value);
    setInputValue(String(value));
  }, [disabled, value]);

  useEffect(() => {
    const sliderElement = containerRef.current;
    if (!sliderElement) return;

    const handleWheel = (event: WheelEvent) => {
      if (disabled || !event.shiftKey) {
        return;
      }

      event.preventDefault();
      const direction = -Math.sign(event.deltaY || event.deltaX);
      const newValue = value + direction * step;
      const roundedNewValue = parseFloat(newValue.toFixed(decimalPlaces));

      const clampedValue = Math.max(min, Math.min(max, roundedNewValue));

      if (clampedValue !== value && !isNaN(clampedValue)) {
        isWheelActivelyChangingRef.current = true;
        setDisplayValue(clampedValue);
        setSmoothValue(clampedValue);

        if (wheelTimeoutRef.current !== undefined) {
          window.clearTimeout(wheelTimeoutRef.current);
        }
        wheelTimeoutRef.current = window.setTimeout(() => {
          isWheelActivelyChangingRef.current = false;
        }, 150);

        const syntheticEvent = {
          target: {
            value: clampedValue,
          },
        };
        onChange(syntheticEvent);
      }
    };

    sliderElement.addEventListener('wheel', handleWheel, { passive: false });

    return () => {
      sliderElement.removeEventListener('wheel', handleWheel);
    };
  }, [disabled, value, min, max, step, onChange, decimalPlaces]);

  // Handle Dragging
  useEffect(() => {
    if (!isDragging || disabled) return;

    const handlePointerMove = (e: MouseEvent | TouchEvent) => {
      let clientX: number;
      let shiftKey: boolean;

      if ('touches' in e) {
        if (e.touches.length === 0) return;
        clientX = e.touches[0].clientX;
        shiftKey = hasFineAdjustmentModifier(e);
        if (e.cancelable) e.preventDefault();
      } else {
        clientX = (e as MouseEvent).clientX;
        shiftKey = hasFineAdjustmentModifier(e);
      }

      const { min: curMin, max: curMax } = rangeRef.current;
      const range = curMax - curMin;
      const geometry = dragGeometryRef.current;
      const thumbXOf = (val: number) =>
        geometry.left + (range !== 0 ? Math.max(0, Math.min(1, (val - curMin) / range)) : 0) * geometry.width;

      let rawValue: number;
      if (shiftKey) {
        if (!fineAnchorRef.current) {
          fineAnchorRef.current = { x: clientX, value: accumulatedValueRef.current };
        }
        const anchor = fineAnchorRef.current;
        rawValue = anchor.value + ((clientX - anchor.x) / geometry.width) * range * FINE_ADJUSTMENT_MULTIPLIER;
      } else {
        if (fineAnchorRef.current) {
          // Re-seat the grab on release so the thumb stays under the pointer
          // instead of snapping to wherever the unmodified mapping would put it.
          geometry.grabOffset = clientX - thumbXOf(accumulatedValueRef.current);
          fineAnchorRef.current = null;
        }
        rawValue = curMin + ((clientX - geometry.grabOffset - geometry.left) / geometry.width) * range;
      }

      accumulatedValueRef.current = Math.max(curMin, Math.min(curMax, rawValue));
      lastPointerXRef.current = clientX;

      const snappedValue = snapToStepRef.current(accumulatedValueRef.current);

      setSmoothValue(accumulatedValueRef.current);
      setDisplayValue(snappedValue);
      onChangeRef.current({ target: { value: snappedValue } });
    };

    const handlePointerUp = () => {
      fineAnchorRef.current = null;
      lastUpTime.current = Date.now();
      pendingTouchRef.current = null;
      suppressTouchChangeRef.current = false;
      if (isDragging) {
        onPointerUp?.();
      }
      setIsDragging(false);
    };

    window.addEventListener('mousemove', handlePointerMove, { passive: false });
    window.addEventListener('mouseup', handlePointerUp);
    window.addEventListener('touchmove', handlePointerMove, { passive: false });
    window.addEventListener('touchend', handlePointerUp);
    window.addEventListener('touchcancel', handlePointerUp);

    return () => {
      window.removeEventListener('mousemove', handlePointerMove);
      window.removeEventListener('mouseup', handlePointerUp);
      window.removeEventListener('touchmove', handlePointerMove);
      window.removeEventListener('touchend', handlePointerUp);
      window.removeEventListener('touchcancel', handlePointerUp);
    };
  }, [disabled, isDragging]);

  useEffect(() => {
    if (isInteracting) {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      return;
    }

    if (isWheelActivelyChangingRef.current) {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      setDisplayValue(value);
      setSmoothValue(value);
      return;
    }

    const startValue = displayValue;
    const endValue = value;
    const duration = 300;
    let startTime: number | null = null;

    const easeInOut = (t: number) => t * t * (3 - 2 * t);

    const animate = (timestamp: number) => {
      if (!startTime) {
        startTime = timestamp;
      }

      const progress = timestamp - startTime;
      const linearFraction = Math.min(progress / duration, 1);
      const easedFraction = easeInOut(linearFraction);
      const currentValue = startValue + (endValue - startValue) * easedFraction;
      setDisplayValue(currentValue);
      setSmoothValue(currentValue);

      if (linearFraction < 1) {
        animationFrameRef.current = requestAnimationFrame(animate);
      }
    };

    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [value, isInteracting]);

  useEffect(() => {
    if (!isEditing) {
      setInputValue(String(value));
    }
  }, [value, isEditing]);

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current?.focus();
      inputRef.current?.select();
    }
  }, [isEditing]);

  const handleReset = () => {
    if (disabled) return;

    const syntheticEvent = {
      target: {
        value: defaultValue,
      },
    };
    onChange(syntheticEvent);
    onPointerUp?.();
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (disabled || suppressTouchChangeRef.current) {
      return;
    }

    if (!isDragging) {
      setDisplayValue(Number(e.target.value));
      setSmoothValue(Number(e.target.value));
      onChange(e);
    }
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLInputElement>) => {
    if (disabled) return;

    if (Date.now() - lastUpTime.current < DOUBLE_CLICK_THRESHOLD_MS) {
      e.preventDefault();
      return;
    }
    e.preventDefault();

    const rect = e.currentTarget.getBoundingClientRect();
    const width = rect.width || 1;
    const range = max - min;
    const thumbX = rect.left + (range !== 0 ? Math.max(0, Math.min(1, (displayValue - min) / range)) : 0) * width;

    // Landing on the thumb picks it up where it sits; landing anywhere else on
    // the track jumps to that point, which is what both references do.
    const grabbedThumb = Math.abs(e.clientX - thumbX) <= THUMB_GRAB_RADIUS_PX;
    const rawValue = grabbedThumb
      ? displayValue
      : min + Math.max(0, Math.min(1, (e.clientX - rect.left) / width)) * range;

    dragGeometryRef.current = { left: rect.left, width, grabOffset: grabbedThumb ? e.clientX - thumbX : 0 };
    fineAnchorRef.current = null;
    accumulatedValueRef.current = rawValue;
    lastPointerXRef.current = e.clientX;

    const snappedValue = snapToStep(rawValue);

    setIsDragging(true);
    setDisplayValue(snappedValue);
    setSmoothValue(snappedValue);
    if (snappedValue !== displayValue) {
      onChange({ target: { value: snappedValue } });
    }
  };

  const handleTouchStart = (e: React.TouchEvent<HTMLInputElement>) => {
    if (disabled) return;

    if (e.touches.length === 0) return;

    const touch = e.touches[0];
    suppressTouchChangeRef.current = true;

    const inputEl = rangeInputRef.current;
    if (!inputEl) return;

    const rect = inputEl.getBoundingClientRect();
    const fraction = max !== min ? (displayValue - min) / (max - min) : 0;
    const thumbX = rect.left + Math.max(0, Math.min(1, fraction)) * rect.width;

    if (Math.abs(touch.clientX - thumbX) > TOUCH_THUMB_HIT_RADIUS_PX) {
      pendingTouchRef.current = null;
      return;
    }

    dragGeometryRef.current = { left: rect.left, width: rect.width || 1, grabOffset: touch.clientX - thumbX };
    fineAnchorRef.current = null;

    pendingTouchRef.current = {
      startX: touch.clientX,
      startY: touch.clientY,
      latestX: touch.clientX,
      startValue: displayValue,
    };
  };

  const handleTouchMove = (e: React.TouchEvent<HTMLInputElement>) => {
    if (disabled) return;

    if (isDragging || !pendingTouchRef.current || e.touches.length === 0) return;

    const touch = e.touches[0];
    const pendingTouch = pendingTouchRef.current;
    pendingTouch.latestX = touch.clientX;

    const deltaX = touch.clientX - pendingTouch.startX;
    const deltaY = touch.clientY - pendingTouch.startY;

    if (Math.abs(deltaY) > TOUCH_DRAG_THRESHOLD_PX && Math.abs(deltaY) > Math.abs(deltaX)) {
      pendingTouchRef.current = null;
      return;
    }

    if (Math.abs(deltaX) < TOUCH_DRAG_THRESHOLD_PX || Math.abs(deltaX) < Math.abs(deltaY)) {
      return;
    }

    const inputEl = rangeInputRef.current;
    if (!inputEl) return;

    const rect = inputEl.getBoundingClientRect();
    const multiplier = hasFineAdjustmentModifier(e) ? FINE_ADJUSTMENT_MULTIPLIER : 1;
    const rawValue = pendingTouch.startValue + (deltaX / rect.width) * (max - min) * multiplier;
    const snappedValue = snapToStep(rawValue);

    accumulatedValueRef.current = rawValue;
    lastPointerXRef.current = touch.clientX;
    pendingTouchRef.current = null;

    if (e.cancelable) {
      e.preventDefault();
    }

    setIsDragging(true);
    setDisplayValue(snappedValue);
    setSmoothValue(snappedValue);
    onChange({ target: { value: snappedValue } });
  };

  const handleTouchEnd = () => {
    pendingTouchRef.current = null;
    suppressTouchChangeRef.current = false;
  };

  const handleValueClick = () => {
    if (disabled) return;

    if (suppressValueClickRef.current) {
      suppressValueClickRef.current = false;
      return;
    }
    setIsEditing(true);
  };

  const handleValueMouseDown = (e: React.MouseEvent<HTMLSpanElement>) => {
    if (disabled || e.button !== 0) return;

    const inputEl = rangeInputRef.current;
    if (!inputEl) return;

    const width = inputEl.getBoundingClientRect().width || 1;
    const startX = e.clientX;
    const startValue = displayValue;
    let scrubbing = false;

    const onMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX;
      if (!scrubbing) {
        if (Math.abs(deltaX) < VALUE_SCRUB_THRESHOLD_PX) return;
        scrubbing = true;
        setIsScrubbingValue(true);
      }

      const { min: curMin, max: curMax } = rangeRef.current;
      const gearing = VALUE_SCRUB_MULTIPLIER * (hasFineAdjustmentModifier(moveEvent) ? FINE_ADJUSTMENT_MULTIPLIER : 1);
      const rawValue = startValue + (deltaX / width) * (curMax - curMin) * gearing;
      const clamped = Math.max(curMin, Math.min(curMax, rawValue));

      accumulatedValueRef.current = clamped;
      const snappedValue = snapToStepRef.current(clamped);

      setSmoothValue(clamped);
      setDisplayValue(snappedValue);
      onChangeRef.current({ target: { value: snappedValue } });
    };

    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      if (!scrubbing) return;

      // The click that follows a scrub would otherwise open the text field.
      suppressValueClickRef.current = true;
      lastUpTime.current = Date.now();
      setIsScrubbingValue(false);
      onPointerUp?.();
    };

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (disabled) return;

    const textVal = e.target.value;
    if (!/^[0-9.,-]*$/.test(textVal)) {
      return;
    }
    setInputValue(textVal);
    const parseableText = textVal.replace(',', '.');
    const parsedValue = parseFloat(parseableText);
    if (!isNaN(parsedValue)) {
      const clampedValue = Math.max(min, Math.min(max, parsedValue));
      onChange({
        target: {
          value: clampedValue,
        },
      });
    }
  };

  const handleInputCommit = () => {
    if (disabled) {
      setInputValue(String(value));
      setIsEditing(false);
      return;
    }

    let newValue = parseFloat(inputValue.replace(',', '.'));
    if (isNaN(newValue)) {
      newValue = value;
    } else {
      newValue = Math.max(min, Math.min(max, newValue));
    }
    const syntheticEvent = {
      target: {
        value: newValue,
      },
    };
    onChange(syntheticEvent);
    setIsEditing(false);
    onPointerUp?.();
  };

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (disabled) return;

    if (e.key === 'Enter') {
      handleInputCommit();
      e.currentTarget.blur();
    } else if (e.key === 'Escape') {
      setInputValue(String(value));
      setIsEditing(false);
      e.currentTarget.blur();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      let currentNum = parseFloat(inputValue.replace(',', '.'));
      if (isNaN(currentNum)) {
        currentNum = value;
      }
      const direction = e.key === 'ArrowUp' ? 1 : -1;
      const newValue = currentNum + direction * step;
      const snappedNewValue = snapToStep(newValue);
      setInputValue(String(snappedNewValue));
      onChange({
        target: {
          value: snappedNewValue,
        },
      });
    }
  };

  const handleRangeKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.ctrlKey || e.metaKey) {
      e.currentTarget.blur();
      return;
    }
    if (GLOBAL_KEYS.includes(e.key)) {
      e.currentTarget.blur();
    }
  };

  const numericValue = isNaN(Number(value)) ? 0 : Number(value);

  return (
    <div
      className={`mb-1.5 group flex items-center gap-2 ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
      ref={containerRef}
    >
      <div
        className={`w-24 shrink-0 grid ${typeof label === 'string' && !disabled ? 'cursor-pointer' : ''}`}
        onClick={typeof label === 'string' && !disabled ? handleReset : undefined}
        onDoubleClick={typeof label === 'string' && !disabled ? handleReset : undefined}
        onMouseEnter={typeof label === 'string' && !disabled ? () => setIsLabelHovered(true) : undefined}
        onMouseLeave={typeof label === 'string' && !disabled ? () => setIsLabelHovered(false) : undefined}
      >
        <span
          aria-hidden={isLabelHovered && typeof label === 'string'}
          className={`col-start-1 row-start-1 text-sm font-medium text-text-secondary select-none truncate transition-opacity duration-200 ease-in-out ${
            isLabelHovered && typeof label === 'string' ? 'opacity-0' : 'opacity-100'
          }`}
        >
          {label}
        </span>
        {typeof label === 'string' && (
          <span
            aria-hidden={!isLabelHovered}
            className={`col-start-1 row-start-1 text-sm font-medium text-text-primary select-none truncate transition-opacity duration-200 ease-in-out pointer-events-none ${
              isLabelHovered ? 'opacity-100' : 'opacity-0'
            }`}
          >
            {t('ui.slider.reset')}
          </span>
        )}
      </div>

      <div className="relative flex-1 h-4">
        <div
          className={`absolute top-1/2 left-0 w-full h-1.5 -translate-y-1/2 rounded-full pointer-events-none ${
            trackClassName || 'bg-card-active'
          }`}
        />
        <div
          className="absolute top-1/2 h-1.5 -translate-y-1/2 rounded-full pointer-events-none bg-accent/25"
          style={{
            left: `${Math.min(fillPercentage, originPercentage)}%`,
            width: `${Math.abs(fillPercentage - originPercentage)}%`,
          }}
        />
        <input
          ref={rangeInputRef}
          className={`absolute top-1/2 left-0 w-full h-7 -translate-y-1/2 appearance-none bg-transparent cursor-pointer m-0 p-0 slider-input z-10 ${
            isDragging ? 'slider-thumb-active' : ''
          } ${disabled ? 'cursor-not-allowed' : ''}`}
          style={{ margin: 0, touchAction: isDragging ? 'none' : 'pan-y' }}
          max={String(max)}
          min={String(min)}
          onChange={handleChange}
          onDoubleClick={handleReset}
          onKeyDown={handleRangeKeyDown}
          onMouseDown={handleMouseDown}
          onTouchStart={handleTouchStart}
          onTouchMove={handleTouchMove}
          onTouchEnd={handleTouchEnd}
          onTouchCancel={handleTouchEnd}
          step={isDragging ? 'any' : String(step)}
          type="range"
          value={thumbValue}
        />
      </div>
      <div className="w-9 shrink-0 text-right">
        {isEditing ? (
          <input
            className="w-full text-sm text-right bg-card-active border border-gray-500 rounded-sm px-1 py-0 outline-none focus:ring-1 focus:ring-blue-500 text-text-primary"
            disabled={disabled}
            max={max}
            min={min}
            onBlur={handleInputCommit}
            onChange={handleInputChange}
            onKeyDown={handleInputKeyDown}
            ref={inputRef}
            step={step}
            type="text"
            value={inputValue}
          />
        ) : (
          <span
            className={`text-sm text-text-primary w-full text-right select-none tabular-nums ${
              disabled ? '' : 'cursor-ew-resize'
            }`}
            onClick={disabled ? undefined : handleValueClick}
            onMouseDown={disabled ? undefined : handleValueMouseDown}
            onDoubleClick={disabled ? undefined : handleReset}
            data-tooltip={disabled ? undefined : t('ui.slider.clickToEdit')}
          >
            {decimalPlaces > 0 && numericValue === 0 ? '0' : numericValue.toFixed(decimalPlaces)}
            {suffix && <span className="text-[10px] align-top inline-block mt-0.5 ml-0.5">{suffix}</span>}
          </span>
        )}
      </div>
    </div>
  );
};

export default Slider;
