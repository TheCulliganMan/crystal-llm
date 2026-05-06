"use client";

import React, { useEffect, useId, useMemo } from "react";
import NextLink from "next/link";
import type { CSSProperties, ElementType } from "react";

// DaisyUI migration policy for this adapter layer.
// - Rendering should use documented Daisy recipes first.
// - Colors come from semantic tokens (`primary`, `secondary`, `error`, `success`, `base-*`, `*-content`) by default.
// - `sx`/`style` are fallback channels only and must stay scoped and minimal.
// - Any hardcoded palette value in migrated surfaces requires an exception entry.
export const DAISY_ELEMENTS_ADAPTER_POLICY = {
  directDaisyPrimitives: true,
  semanticColorTokens: true,
  scopedStyleFallback: true,
  hardcodeExceptionsRequired: true,
  themeReadabilityMustVerify: true,
} as const;

export const DAISY_THEME_ACCEPTANCE_CRITERIA = {
  colorTokensPreferred: ["primary", "secondary", "accent", "base-content", "base-100", "base-200", "error", "warning", "info", "success"],
  noHardcodedPaletteDefaults: true,
  themeThemeBrandReadabilityChecks: ["default", "sunset", "brand variants present in data-brand-theme"],
} as const;

type Responsive<T> = T | { xs?: T; sm?: T; md?: T; lg?: T; xl?: T };
type SxValue =
  | false
  | null
  | number
  | string
  | CSSProperties
  | { [key: string]: unknown }
  | ReadonlyArray<SxValue>;

type SxRecord = Record<string, unknown>;

type KnownBreakpointKeys = "xs" | "sm" | "md" | "lg" | "xl";
export type ThemeOptions = {
  palette?: Record<string, unknown>;
  typography?: Record<string, unknown>;
  breakpoints?: {
    values?: Record<KnownBreakpointKeys, number>;
    down?: (key: KnownBreakpointKeys | string | number) => string;
  };
  [key: string]: unknown;
};

type Theme = ThemeOptions;

const defaultBreakpointValues: Record<KnownBreakpointKeys, number> = {
  xs: 0,
  sm: 600,
  md: 900,
  lg: 1200,
  xl: 1536,
};

const buildDefaultBreakpoint = (values: Record<KnownBreakpointKeys, number>) => ({
  values,
  down: (key: KnownBreakpointKeys | string | number) => {
    if (typeof key === "number") {
      return `(max-width: ${key}px)`;
    }
    const numeric = typeof key === "string" ? values[key as KnownBreakpointKeys] : undefined;
    if (numeric == null) {
      return "(min-width: 0px)";
    }
    return `(max-width: ${numeric - 0.05}px)`;
  },
});

const defaultTheme: Theme = {
  breakpoints: buildDefaultBreakpoint(defaultBreakpointValues),
};

const ThemeContext = React.createContext<Theme>(defaultTheme);

const isRecord = (value: unknown): value is SxRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const resolveResponsive = <T,>(value: Responsive<T> | undefined): T | undefined => {
  if (!isRecord(value)) {
    return value as T | undefined;
  }
  const bucket = value as Record<string, T>;
  const resolved = bucket.xs ?? bucket.sm ?? bucket.md ?? bucket.lg ?? bucket.xl;
  if (resolved !== undefined && resolved !== null) {
    return resolved;
  }

  for (const key in bucket) {
    if (Object.prototype.hasOwnProperty.call(bucket, key)) {
      return bucket[key];
    }
  }

  return undefined;
};

const spacingKeys = new Set<string>([
  "p",
  "m",
  "px",
  "py",
  "pt",
  "pr",
  "pb",
  "pl",
  "mx",
  "my",
  "mt",
  "mr",
  "mb",
  "ml",
  "w",
  "h",
  "minW",
  "minH",
  "maxW",
  "maxH",
  "width",
  "height",
  "minWidth",
  "minHeight",
  "maxWidth",
  "maxHeight",
]);

const spacingValue = (value: number | string): string => {
  if (typeof value === "number") {
    return value === 0 ? "0" : `${value * 0.5}rem`;
  }
  return value;
};

const applySpacing = (style: CSSProperties, key: string, rawValue: unknown) => {
  const value = isRecord(rawValue) ? resolveResponsive(rawValue as Responsive<string | number>) : rawValue;
  if (typeof value !== "number" && typeof value !== "string") {
    return;
  }
  const next = spacingValue(value);

  if (key === "p") {
    style.padding = next;
    return;
  }
  if (key === "m") {
    style.margin = next;
    return;
  }
  if (key === "px") {
    style.paddingLeft = next;
    style.paddingRight = next;
    return;
  }
  if (key === "py") {
    style.paddingTop = next;
    style.paddingBottom = next;
    return;
  }
  if (key === "pt") {
    style.paddingTop = next;
    return;
  }
  if (key === "pr") {
    style.paddingRight = next;
    return;
  }
  if (key === "pb") {
    style.paddingBottom = next;
    return;
  }
  if (key === "pl") {
    style.paddingLeft = next;
    return;
  }
  if (key === "mx") {
    style.marginLeft = next;
    style.marginRight = next;
    return;
  }
  if (key === "my") {
    style.marginTop = next;
    style.marginBottom = next;
    return;
  }
  if (key === "mt") {
    style.marginTop = next;
    return;
  }
  if (key === "mr") {
    style.marginRight = next;
    return;
  }
  if (key === "mb") {
    style.marginBottom = next;
    return;
  }
  if (key === "ml") {
    style.marginLeft = next;
    return;
  }
  if (key === "w" || key === "width") {
    style.width = next;
    return;
  }
  if (key === "h" || key === "height") {
    style.height = next;
    return;
  }
  if (key === "minW" || key === "minWidth") {
    style.minWidth = next;
    return;
  }
  if (key === "minH" || key === "minHeight") {
    style.minHeight = next;
    return;
  }
  if (key === "maxW" || key === "maxWidth") {
    style.maxWidth = next;
    return;
  }
  if (key === "maxH" || key === "maxHeight") {
    style.maxHeight = next;
    return;
  }
};

const sxToStyle = (sx?: SxValue): CSSProperties => {
  if (sx == null || sx === false) {
    return {};
  }

  const sources = Array.isArray(sx) ? sx : [sx];
  const out: CSSProperties = {};

  for (const source of sources) {
    if (source == null || source === false) {
      continue;
    }
    if (typeof source === "number" || typeof source === "string") {
      continue;
    }
    if (!isRecord(source)) {
      continue;
    }

    for (const [rawKey, rawValue] of Object.entries(source)) {
      if (!rawKey || rawValue == null) {
        continue;
      }
      if (rawKey.startsWith("&") || rawKey.startsWith(":") || rawKey.startsWith("@")) {
        continue;
      }

      if (spacingKeys.has(rawKey)) {
        applySpacing(out, rawKey, rawValue);
        continue;
      }

      const value = isRecord(rawValue)
        ? resolveResponsive(rawValue as Responsive<string | number | boolean>)
        : rawValue;

      if (value == null || typeof value === "object") {
        continue;
      }
      (out as Record<string, string | number | boolean>)[rawKey] = value as
        string | number | boolean;
    }
  }

  return out;
};

const toClassName = (...tokens: Array<string | undefined | false | null>) =>
  tokens.filter(Boolean).join(" ");

const stripFullWidthProps = <T extends Record<string, unknown>>(props: T): T => {
  const cleanedProps = { ...props };
  delete cleanedProps.fullWidth;
  delete cleanedProps.fullwidth;
  return cleanedProps;
};

const resolveAlignClass = (value?: React.CSSProperties["alignItems"]): string | undefined => {
  if (!value) {
    return undefined;
  }
  if (value === "flex-start") {
    return "items-start";
  }
  if (value === "flex-end") {
    return "items-end";
  }
  if (value === "stretch") {
    return "items-stretch";
  }
  if (value === "baseline") {
    return "items-baseline";
  }
  return value === "center" ? "items-center" : undefined;
};

const resolveJustifyClass = (value?: React.CSSProperties["justifyContent"]): string | undefined => {
  if (!value) {
    return undefined;
  }
  if (value === "flex-start") {
    return "justify-start";
  }
  if (value === "flex-end") {
    return "justify-end";
  }
  return value === "center" ? "justify-center" : undefined;
};

const resolveGapClass = (spacing: number | Responsive<number> | undefined): string | undefined => {
  const value = resolveResponsive(spacing);
  if (value == null) {
    return undefined;
  }
  if (Number.isInteger(value)) {
    return `gap-${value}`;
  }
  return `gap-[${value * 0.25}rem]`;
};

const normalizeColorClass = (color?: string | null): string | undefined => {
  switch (color) {
    case "primary":
      return "text-primary";
    case "accent":
      return "text-accent";
    case "secondary":
      return "text-secondary";
    case "info":
      return "text-info";
    case "warning":
      return "text-warning";
    case "success":
      return "text-success";
    case "error":
      return "text-error";
    case "text.primary":
      return "text-primary-content";
    case "text.secondary":
      return "text-base-content/70";
    case "text.disabled":
      return "text-base-content/40";
    default:
      return undefined;
  }
};

const normalizeButtonTone = (color?: string | null): string | undefined => {
  switch (color) {
    case "primary":
      return "btn-primary";
    case "secondary":
      return "btn-secondary";
    case "accent":
      return "btn-accent";
    case "neutral":
      return "btn-neutral";
    case "warning":
      return "btn-warning";
    case "error":
      return "btn-error";
    case "success":
      return "btn-success";
    case "info":
      return "btn-info";
    default:
      return undefined;
  }
};

const normalizeBadgeToneClass = (color?: string | null): string | undefined => {
  switch (color) {
    case "primary":
      return "badge-primary";
    case "secondary":
      return "badge-secondary";
    case "accent":
      return "badge-accent";
    case "neutral":
      return "badge-neutral";
    case "warning":
      return "badge-warning";
    case "error":
      return "badge-error";
    case "success":
      return "badge-success";
    case "info":
      return "badge-info";
    default:
      return undefined;
  }
};

type SxProps = {
  sx?: SxValue;
  className?: string;
  style?: CSSProperties;
  component?: ElementType;
  as?: ElementType;
  [key: string]: unknown;
};

type DirectionValue = "column" | "row" | Responsive<"column" | "row">;

type ContainerProps = React.HTMLAttributes<HTMLDivElement> & SxProps;

export const Container = ({ className, sx, style, ...props }: ContainerProps) => (
  <div
    className={toClassName("mx-auto w-full px-2", className)}
    style={{ ...sxToStyle(sx), ...style }}
    {...props}
  />
);

export const Box = ({
  component,
  className,
  sx,
  style,
  ref,
  ...props
}: SxProps & React.HTMLAttributes<HTMLElement> & { component?: ElementType; ref?: React.Ref<HTMLElement> }) => {
  const Tag = (component as ElementType) ?? "div";
  return (
    <Tag
      className={className}
      ref={ref}
      style={{ ...sxToStyle(sx), ...style }}
      {...props}
    />
  );
};

export const Stack = ({
  direction = "column",
  spacing = 0,
  alignItems,
  justifyContent,
  flexWrap,
  component = "div",
  className,
  sx,
  style,
  useFlexGap,
  children,
  fullWidth,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & SxProps & {
  direction?: DirectionValue;
  spacing?: number | Responsive<number>;
  alignItems?: React.CSSProperties["alignItems"] | Responsive<React.CSSProperties["alignItems"]>;
  justifyContent?: React.CSSProperties["justifyContent"] | Responsive<React.CSSProperties["justifyContent"]>;
  flexWrap?: React.CSSProperties["flexWrap"];
  component?: ElementType;
  fullWidth?: boolean;
  useFlexGap?: boolean;
}) => {
  const resolvedDirection = typeof direction === "string" ? direction : resolveResponsive(direction) ?? "column";
  const resolvedAlign = alignItems == null || typeof alignItems === "string" ? alignItems : resolveResponsive(alignItems);
  const resolvedJustify =
    justifyContent == null || typeof justifyContent === "string" ? justifyContent : resolveResponsive(justifyContent);
  const resolvedSpacing = typeof spacing === "number" ? spacing : resolveResponsive(spacing);

  const Tag = component as ElementType;

  return (
    <Tag
      className={toClassName(
        "flex",
        resolvedDirection === "row" ? "flex-row" : "flex-col",
        resolveAlignClass(resolvedAlign),
        resolveJustifyClass(resolvedJustify),
        flexWrap === "wrap" ? "flex-wrap" : undefined,
        (useFlexGap ?? true) ? (resolvedSpacing ? resolveGapClass(resolvedSpacing) : undefined) : undefined,
        fullWidth ? "w-full" : undefined,
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...props}
    >
      {children}
    </Tag>
  );
};

const TYPOGRAPHY_VARIANTS: Record<string, string> = {
  h1: "text-4xl font-bold",
  h2: "text-3xl font-semibold",
  h3: "text-2xl font-semibold",
  h4: "text-xl font-semibold",
  h5: "text-lg font-medium",
  h6: "text-base font-medium",
  subtitle1: "text-base font-semibold",
  subtitle2: "text-sm font-semibold",
  body1: "text-base",
  body2: "text-sm",
  caption: "text-xs uppercase tracking-[0.2em]",
  overline: "text-xs uppercase tracking-[0.28em]",
};

export const Typography = ({
  variant,
  color,
  component,
  className,
  sx,
  style,
  children,
  ref,
  display,
  ...props
}: React.HTMLAttributes<HTMLElement> & {
  variant?:
    | "h1"
    | "h2"
    | "h3"
    | "h4"
    | "h5"
    | "h6"
    | "subtitle1"
    | "subtitle2"
    | "body2"
    | "caption"
    | "body1"
    | "overline";
  color?: string;
  component?: ElementType;
  sx?: SxValue;
  display?: React.CSSProperties["display"];
  ref?: React.Ref<HTMLElement>;
}) => {
  const Tag = component
    ? component
    : variant === "h1"
      ? "h1"
      : variant === "h2"
        ? "h2"
        : variant === "h3"
          ? "h3"
          : variant === "h4"
            ? "h4"
            : variant === "h5"
              ? "h5"
              : variant === "h6"
                ? "h6"
                : "p";

  return (
    <Tag
      className={toClassName(normalizeColorClass(color), TYPOGRAPHY_VARIANTS[variant ?? "body1"], className)}
      ref={ref}
      style={{ ...sxToStyle(sx), ...style, display }}
      {...props}
    >
      {children}
    </Tag>
  );
};

export const Paper = ({
  variant,
  className,
  sx,
  style,
  children,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & {
  variant?: "outlined" | "elevation";
  sx?: SxValue;
  children?: React.ReactNode;
  ref?: React.Ref<HTMLDivElement>;
}) => (
  <div
    className={toClassName("card bg-base-100 text-base-content", variant === "outlined" ? "card-bordered" : "shadow-sm", className)}
    style={{ ...sxToStyle(sx), ...style }}
    ref={ref}
    {...props}
  >
    {children}
  </div>
);

export const Card = Paper;

export const CardContent = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { sx?: SxValue; children?: React.ReactNode; component?: ElementType }) => (
  <div className={toClassName("card-body", className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </div>
);

export const CardActionArea = ({
  className,
  sx,
  style,
  children,
  fullWidth = true,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { sx?: SxValue; fullWidth?: boolean }) => {
  const cleanCardActionProps = stripFullWidthProps(props as Record<string, unknown>);

  return (
    <button
      type="button"
      className={toClassName("btn btn-ghost justify-start shadow-none", fullWidth ? "w-full" : undefined, className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...(cleanCardActionProps as React.ButtonHTMLAttributes<HTMLButtonElement>)}
    >
      {children}
    </button>
  );
};

export const Alert = ({
  severity = "info",
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & {
  severity?: "error" | "warning" | "success" | "info";
  sx?: SxValue;
}) => {
  const severityClass =
    severity === "error"
      ? "alert-error"
      : severity === "warning"
        ? "alert-warning"
        : severity === "success"
          ? "alert-success"
          : "alert-info";

  return (
    <div
      role="status"
      className={toClassName("alert", severityClass, className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...props}
    >
      {children}
    </div>
  );
};

export const Chip = ({
  size,
  variant,
  color,
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & {
  size?: "small" | "medium";
  variant?: "outlined" | "filled";
  color?: string;
  sx?: SxValue;
  label?: React.ReactNode;
}) => (
  <span
    className={toClassName(
      "badge",
      size === "small" ? "badge-sm" : "badge-md",
      variant === "outlined" ? "badge-outline" : undefined,
      normalizeBadgeToneClass(color),
      className,
    )}
    style={{ ...sxToStyle(sx), ...style }}
    {...props}
  >
    {children ?? props.label}
  </span>
);

export const Divider = ({
  className,
  sx,
  style,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { sx?: SxValue }) => (
  <div className={toClassName("divider", className)} style={{ ...sxToStyle(sx), ...style }} {...props} />
);

export const LinearProgress = ({
  value = 0,
  variant,
  className,
  sx,
  style,
  ...props
}: React.ProgressHTMLAttributes<HTMLProgressElement> & {
  value?: number;
  variant?: "determinate" | "indeterminate";
  sx?: SxValue;
}) => (
  <progress
    className={toClassName("progress progress-primary w-full", className)}
    value={variant === "determinate" ? Math.max(0, Math.min(100, value)) : undefined}
    max={100}
    style={{ ...sxToStyle(sx), ...style }}
    {...props}
  />
);

export const CircularProgress = ({ size = 24 }: { size?: number }) => (
  <span className="loading loading-spinner loading-sm" style={{ width: size, height: size }} aria-hidden="true" />
);

export const TextField = ({
  label,
  helperText,
  error,
  size,
  sx,
  className,
  disabled,
  style,
  select,
  multiline,
  rows,
  minRows,
  fullWidth,
  value,
  onChange,
  children,
  inputProps,
  type,
  ...props
}: Omit<React.InputHTMLAttributes<HTMLInputElement>, "size">
    & Omit<React.TextareaHTMLAttributes<HTMLTextAreaElement>, "value" | "onChange" | "rows" | "size">
    & Omit<React.SelectHTMLAttributes<HTMLSelectElement>, "value" | "onChange" | "size"> & {
      label?: string;
      helperText?: string;
      error?: boolean;
      size?: "small" | "medium";
      sx?: SxValue;
      select?: boolean;
      multiline?: boolean;
      rows?: number;
      minRows?: number;
      inputProps?: Record<string, string | number | boolean | undefined>;
      children?: React.ReactNode;
      value?: string | number;
      onChange?: React.ChangeEventHandler<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>;
      type?: string;
      fullWidth?: boolean;
}) => {
  const id = useId();
  const controlId = `${id}-input`;
  const inputFieldProps = stripFullWidthProps(props as Record<string, unknown>);

  const resolvedValue = value == null ? "" : `${value}`;
  const controlStateClass = error
    ? select
      ? "select-error"
      : multiline
        ? "textarea-error"
        : "input-error"
    : undefined;

  const sizeClass = size === "small"
    ? select
      ? "select-sm"
      : multiline
        ? "textarea-sm"
        : "input-sm"
    : undefined;

  const controlClass = toClassName(
    select
      ? "select select-bordered"
      : multiline
        ? "textarea textarea-bordered"
        : "input input-bordered",
    controlStateClass,
    fullWidth ? "w-full" : undefined,
    sizeClass,
    className,
  );

  const controlStyle = { ...sxToStyle(sx), ...(style ?? {}) };

  const control =
    select
      ? (
        <select
          className={controlClass}
          id={controlId}
          name={props.name}
          disabled={disabled}
          {...inputProps}
          value={resolvedValue}
          onChange={onChange as React.ChangeEventHandler<HTMLSelectElement>}
          style={controlStyle}
          {...inputFieldProps}
        >
          {children}
        </select>
      )
      : multiline
        ? (
          <textarea
            className={controlClass}
            id={controlId}
            name={props.name}
            rows={rows ?? minRows ?? 3}
            disabled={disabled}
            {...inputProps}
            value={resolvedValue}
            onChange={onChange as React.ChangeEventHandler<HTMLTextAreaElement>}
            style={controlStyle}
            {...inputFieldProps}
          />
        )
        : (
          <input
            className={controlClass}
            id={controlId}
            type={type ?? "text"}
            name={props.name}
            disabled={disabled}
            {...inputProps}
            value={resolvedValue}
            onChange={onChange as React.ChangeEventHandler<HTMLInputElement>}
            style={controlStyle}
            aria-label={label}
            {...inputFieldProps}
          />
        );

  return (
    <div className="form-control gap-2">
      {label ? (
        <label className="label" htmlFor={controlId}>
          <span className="label-text">{label}</span>
        </label>
      ) : null}
      {control}
      {helperText ? (
        <label className={toClassName("label", error ? "text-error" : undefined)} htmlFor={controlId}>
          <span className="label-text-alt">{helperText}</span>
        </label>
      ) : null}
    </div>
  );
};

export const FormControl = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { sx?: SxValue }) => (
  <div className={toClassName("form-control", className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </div>
);

export const InputLabel = ({ className, ...props }: React.LabelHTMLAttributes<HTMLLabelElement>) => (
  <label className={toClassName("label", className)} {...props} />
);

export const Select = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.SelectHTMLAttributes<HTMLSelectElement> & { sx?: SxValue }) => (
  <select
    className={toClassName("select select-bordered w-full", className)}
    style={{ ...sxToStyle(sx), ...style }}
    {...props}
  >
    {children}
  </select>
);

export const MenuItem = ({
  value,
  children,
  ...props
}: React.OptionHTMLAttributes<HTMLOptionElement> & { value: string }) => (
  <option value={value} {...props}>
    {children}
  </option>
);

export const Autocomplete = <T,>({
  options = [],
  value = null,
  onChange,
  getOptionLabel = (option: T) => `${option}`,
  renderInput,
  disabled,
  sx,
  className,
  label,
  helperText,
  size,
  ...props
}: {
  options?: readonly T[];
  value?: T | null;
  onChange?: (_event: React.SyntheticEvent, next: T | null) => void;
  getOptionLabel?: (value: T) => string;
  renderInput?: (params: Record<string, unknown>) => React.ReactElement;
  disabled?: boolean;
  sx?: SxValue;
  className?: string;
  label?: string;
  helperText?: string;
  size?: "small" | "medium";
} & Record<string, unknown>) => {
  const datalistId = useId();
  const [inputValue, setInputValue] = React.useState(() => (value == null ? "" : getOptionLabel(value)));

  const labels = useMemo(() => options.map((option) => getOptionLabel(option)), [options, getOptionLabel]);
  const resolvedValue = useMemo(() => (value == null ? "" : getOptionLabel(value as T)), [value, getOptionLabel]);

  useEffect(() => {
    setInputValue(resolvedValue);
  }, [resolvedValue]);

  const changeValue = (next: string) => {
    setInputValue(next);
    const exact = options.find((option) => getOptionLabel(option) === next) ?? null;
    onChange?.(new Event("change") as unknown as React.SyntheticEvent, exact as T | null);
  };

  const renderedInput = renderInput?.({
    id: datalistId,
    value: inputValue,
    disabled,
    label,
    size,
    helperText,
    onChange: (event: React.ChangeEvent<HTMLInputElement>) => {
      const next = (event.target as HTMLInputElement).value;
      changeValue(next);
    },
    inputProps: {
      list: datalistId,
      onBlur: () => setInputValue(resolvedValue),
    },
  }) ?? null;

  if (renderedInput) {
    return (
      <Box sx={sx} className={className}>
        {renderedInput}
        <datalist id={datalistId}>
          {labels.map((labelItem) => (
            <option key={labelItem} value={labelItem} />
          ))}
        </datalist>
      </Box>
    );
  }

  return (
    <TextField
      label={label}
      value={inputValue}
      onChange={(event) => changeValue((event.target as HTMLInputElement).value)}
      size={size}
      sx={sx}
      className={className}
      disabled={disabled}
      helperText={helperText}
      inputProps={{ list: datalistId }}
      {...props}
    >
      {null}
    </TextField>
  );
};

export const TableContainer = ({
  className,
  sx,
  style,
  component,
  variant,
  children,
  ...props
}: SxProps & {
  component?: ElementType;
  variant?: "outlined" | "default";
}) => {
  const Tag = (component as ElementType) ?? "div";
  return (
    <Tag
      className={toClassName(
        "overflow-x-auto",
        variant === "outlined" ? "rounded-box border border-base-300" : undefined,
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...props}
    >
      {children}
    </Tag>
  );
};

export const Table = ({
  className,
  sx,
  style,
  children,
  size = "small",
  ...props
}: React.TableHTMLAttributes<HTMLTableElement> & {
  sx?: SxValue;
  size?: "small" | "medium";
}) => (
  <table
    className={toClassName("table", size === "small" ? "table-sm" : undefined, className)}
    style={{ ...sxToStyle(sx), ...style }}
    {...props}
  >
    {children}
  </table>
);

export const TableHead = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLTableSectionElement> & { sx?: SxValue }) => (
  <thead className={toClassName("text-left", className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </thead>
);

export const TableBody = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLTableSectionElement> & { sx?: SxValue }) => (
  <tbody className={className} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </tbody>
);

export const TableRow = ({
  className,
  sx,
  style,
  children,
  hover,
  selected,
  ...props
}: React.HTMLAttributes<HTMLTableRowElement> & {
  sx?: SxValue;
  hover?: boolean;
  selected?: boolean;
}) => (
  <tr
    className={toClassName(
      className,
      hover ? "hover" : undefined,
      selected ? "bg-base-200/50" : undefined,
    )}
    style={{ ...sxToStyle(sx), ...style }}
    {...props}
  >
    {children}
  </tr>
);

export const TableCell = ({
  component,
  align,
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLTableCellElement> & {
  sx?: SxValue;
  component?: "th" | "td";
  align?: "right" | "left" | "center";
  colSpan?: number;
}) => {
  const Tag = component === "th" ? "th" : "td";
  const alignClass =
    align === "right" ? "text-right" : align === "center" ? "text-center" : align === "left" ? "text-left" : undefined;

  return (
    <Tag className={toClassName(alignClass, className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
      {children}
    </Tag>
  );
};

export const Accordion = ({
  className,
  children,
  ...props
}: React.DetailsHTMLAttributes<HTMLDetailsElement>) => (
  <details className={toClassName("collapse collapse-arrow", className)} {...props}>
    {children}
  </details>
);

type CollapseProps = React.DetailsHTMLAttributes<HTMLDetailsElement> & {
  title: React.ReactNode;
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  titleClassName?: string;
  contentClassName?: string;
  sx?: SxValue;
};

export const Collapse = ({
  title,
  titleClassName,
  contentClassName,
  open,
  defaultOpen = false,
  onOpenChange,
  className,
  sx,
  style,
  children,
  ...props
}: CollapseProps) => {
  const controlled = open !== undefined;
  const [internalOpen, setInternalOpen] = React.useState(defaultOpen);
  const resolvedOpen = controlled ? open : internalOpen;

  const handleToggle = (event: React.SyntheticEvent<HTMLDetailsElement>) => {
    const next = event.currentTarget.open;
    if (!controlled) {
      setInternalOpen(next);
    }
    onOpenChange?.(next);
  };

  return (
    <details
      className={toClassName("collapse collapse-plus border border-base-300 bg-base-200", className)}
      style={{ ...sxToStyle(sx), ...style }}
      open={resolvedOpen}
      onToggle={handleToggle}
      {...props}
    >
      <summary className={toClassName("collapse-title text-sm font-semibold", titleClassName)}>{title}</summary>
      <div className={toClassName("collapse-content pt-0", contentClassName)}>{children}</div>
    </details>
  );
};

export const CollapseSummary = ({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={toClassName("collapse-title", className)} {...props}>
    {children}
  </div>
);

export const CollapseDetails = ({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={toClassName("collapse-content", className)} {...props}>
    {children}
  </div>
);

export const Dialog = ({
  open,
  onClose,
  fullWidth = false,
  fullScreen = false,
  maxWidth = undefined,
  className,
  sx,
  style,
  children,
  ...props
}: Omit<React.DialogHTMLAttributes<HTMLDialogElement>, "open" | "onClose"> & {
  open?: boolean;
  onClose?: () => void;
  sx?: SxValue;
  fullWidth?: boolean;
  fullScreen?: boolean;
  maxWidth?: string | false;
}) => {
  if (!open) {
    return null;
  }

  const dialogMaxWidthClass =
    maxWidth === "xs"
      ? "max-w-xs"
      : maxWidth === "sm"
        ? "max-w-sm"
        : maxWidth === "md"
          ? "max-w-md"
          : maxWidth === "lg"
            ? "max-w-lg"
            : maxWidth === "xl"
              ? "max-w-xl"
              : maxWidth === false
                ? "max-w-full w-full"
                : undefined;

  const handleDialogClose = () => {
    onClose?.();
  };
  const dialogProps = stripFullWidthProps(props as Record<string, unknown>);

  return (
    <dialog
      open
      onClose={handleDialogClose}
      role="dialog"
      aria-modal="true"
      className={toClassName("modal modal-open", className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...(dialogProps as Omit<
        React.DialogHTMLAttributes<HTMLDialogElement>,
        "open" | "onClose" | "fullWidth" | "fullScreen" | "maxWidth"
      >)}
    >
      <div
        className={toClassName(
          "modal-box h-auto max-h-[90vh] overflow-y-auto",
          fullWidth || maxWidth === false ? "w-full max-w-full" : undefined,
          fullScreen ? "w-screen h-screen max-h-screen" : undefined,
          dialogMaxWidthClass,
        )}
      >
        {children}
      </div>
      <form className="modal-backdrop" method="dialog" onSubmit={handleDialogClose}>
        <button onClick={handleDialogClose} type="submit" aria-label="Close dialog" />
      </form>
    </dialog>
  );
};

export const DialogTitle = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement> & { sx?: SxValue }) => (
  <h3 className={toClassName("font-bold text-lg", className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </h3>
);

export const DialogContent = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { sx?: SxValue }) => (
  <div className={toClassName("mt-2", className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </div>
);

export const DialogActions = ({
  className,
  sx,
  style,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { sx?: SxValue }) => (
  <div className={toClassName("modal-action mt-4", className)} style={{ ...sxToStyle(sx), ...style }} {...props}>
    {children}
  </div>
);

export const Link = ({
  href,
  children,
  className,
  ...props
}: React.AnchorHTMLAttributes<HTMLAnchorElement> & { href?: string }) => {
  if (!href || /^#/.test(href)) {
    return (
      <a className={toClassName("link link-hover", className)} {...props}>
        {children}
      </a>
    );
  }

  const isExternal = href.startsWith("http") || href.startsWith("mailto:") || href.startsWith("tel:");
  if (isExternal) {
    return (
      <a
        className={toClassName("link link-hover", className)}
        href={href}
        {...(props as React.AnchorHTMLAttributes<HTMLAnchorElement>)}
      >
        {children}
      </a>
    );
  }

  return (
    <NextLink href={href} className={toClassName("link link-hover", className)} {...props}>
      {children}
    </NextLink>
  );
};

export const Button = ({
  variant = "contained",
  size,
  component,
  href,
  color,
  className,
  sx,
  style,
  disabled,
  fullWidth = false,
  type = "button",
  download,
  children,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  component?: ElementType;
  href?: string;
  color?: "primary" | "secondary" | "accent" | "neutral" | "info" | "warning" | "error" | "success" | "inherit" | string;
  variant?: "contained" | "outlined" | "text" | "ghost" | "link";
  size?: "small" | "medium" | "large";
  sx?: SxValue;
  fullWidth?: boolean;
  download?: string | boolean;
}) => {
  const cleanButtonProps = stripFullWidthProps(props as Record<string, unknown>);

  const toneClass = normalizeButtonTone(color);
  const variantClass =
    variant === "outlined"
      ? "btn-outline"
      : variant === "text" || variant === "ghost"
        ? "btn-ghost"
        : variant === "link"
          ? "btn-link"
          : "btn";
  const defaultToneClass = variant === "contained" && !toneClass ? "btn-primary" : undefined;
  const sizeClass = size === "small" ? "btn-sm" : size === "large" ? "btn-lg" : undefined;

  const btnClass = toClassName(
    variantClass,
    toneClass,
    defaultToneClass,
    sizeClass,
    fullWidth ? "w-full" : undefined,
    className,
    disabled ? "btn-disabled" : undefined,
  );
  const resolvedStyle = { ...sxToStyle(sx), ...style };

  if (component && component !== "a") {
    const Component = component as React.ElementType;
    return (
      <Component
        className={btnClass}
        style={resolvedStyle}
        aria-disabled={disabled}
        {...cleanButtonProps}
      >
        {children}
      </Component>
    );
  }

  if (component === "a" || href) {
    if (href?.startsWith("http") || href?.startsWith("mailto:") || href?.startsWith("tel:") || href?.startsWith("/")) {
      return (
        <a
          className={btnClass}
          href={href}
          download={download as string | undefined}
          style={resolvedStyle}
          aria-disabled={disabled}
          tabIndex={disabled ? -1 : undefined}
          {...(cleanButtonProps as React.AnchorHTMLAttributes<HTMLAnchorElement>)}
        >
          {children}
        </a>
      );
    }

    return (
      <NextLink
        href={href ?? "#"}
        className={btnClass}
        style={resolvedStyle}
        aria-disabled={disabled}
        tabIndex={disabled ? -1 : undefined}
      >
        {children}
      </NextLink>
    );
  }

  return (
    <button
      type={type as React.ButtonHTMLAttributes<HTMLButtonElement>["type"]}
      className={btnClass}
      disabled={disabled}
      style={resolvedStyle}
      {...cleanButtonProps}
    >
      {children}
    </button>
  );
};

export const useTheme = () => React.useContext(ThemeContext);
export const createTheme = (themeOptions?: ThemeOptions): Theme => {
  const options = themeOptions ?? {};
  const breakpointValues = options.breakpoints?.values ?? defaultBreakpointValues;
  const breakpoints = {
    values: breakpointValues,
    down: options.breakpoints?.down ?? ((key) => buildDefaultBreakpoint(breakpointValues).down(key)),
  };
  return {
    ...options,
    breakpoints,
  };
};
export const ThemeProvider = ({
  children,
  theme,
}: {
  children: React.ReactNode;
  theme?: Theme;
}) => <ThemeContext.Provider value={theme ?? defaultTheme}>{children}</ThemeContext.Provider>;

export const useMediaQuery = (
  query: string,
  _options?: {
    noSsr?: boolean;
    defaultMatches?: boolean;
  },
) => {
  if (typeof window === "undefined") {
    return Boolean(_options?.defaultMatches);
  }
  return window.matchMedia(query).matches;
};

export const CssBaseline = () => null;
export const __unused = { React };
