/** Appearance prefs: apply theme / font / density to the document root. */

export type AppearanceTheme = "dark" | "light" | "system";
export type AppearanceFontSize = "small" | "medium" | "large";
export type AppearanceDensity = "comfortable" | "compact";

/** Hot tabs in the titlebar cluster (not Projects / sidebar-only pages). */
export type TopNavbarTabId = "chat" | "work" | "docs" | "sprint";

export type TopNavbarTabDef = {
	id: TopNavbarTabId;
	label: string;
	path: string;
};

/** Canonical order for the top hot navbar. */
export const TOP_NAVBAR_TAB_DEFS: readonly TopNavbarTabDef[] = [
	{ id: "chat", label: "Chat", path: "/agent-chat" },
	{ id: "work", label: "Work", path: "/" },
	{ id: "docs", label: "Docs", path: "/docs" },
	{ id: "sprint", label: "Sprint", path: "/sprint" },
] as const;

export const TOP_NAVBAR_TAB_IDS: readonly TopNavbarTabId[] =
	TOP_NAVBAR_TAB_DEFS.map((t) => t.id);

export type TopNavbarTabsPrefs = Record<TopNavbarTabId, boolean>;

export type AppearancePrefs = {
	theme: AppearanceTheme;
	fontSize: AppearanceFontSize;
	density: AppearanceDensity;
	/** Which hot tabs appear in the titlebar; at least one must stay on. */
	topNavbarTabs: TopNavbarTabsPrefs;
};

export const APPEARANCE_STORAGE_KEY = "openmesh.appearance";

export const DEFAULT_TOP_NAVBAR_TABS: TopNavbarTabsPrefs = {
	chat: true,
	work: true,
	docs: true,
	sprint: true,
};

export const DEFAULT_APPEARANCE: AppearancePrefs = {
	theme: "dark",
	fontSize: "medium",
	density: "comfortable",
	topNavbarTabs: { ...DEFAULT_TOP_NAVBAR_TABS },
};

const THEMES = new Set<AppearanceTheme>(["dark", "light", "system"]);
const FONT_SIZES = new Set<AppearanceFontSize>(["small", "medium", "large"]);
const DENSITIES = new Set<AppearanceDensity>(["comfortable", "compact"]);
const TAB_ID_SET = new Set<TopNavbarTabId>(TOP_NAVBAR_TAB_IDS);

function asTheme(v: unknown): AppearanceTheme {
	return typeof v === "string" && THEMES.has(v as AppearanceTheme)
		? (v as AppearanceTheme)
		: DEFAULT_APPEARANCE.theme;
}

function asFontSize(v: unknown): AppearanceFontSize {
	return typeof v === "string" && FONT_SIZES.has(v as AppearanceFontSize)
		? (v as AppearanceFontSize)
		: DEFAULT_APPEARANCE.fontSize;
}

function asDensity(v: unknown): AppearanceDensity {
	return typeof v === "string" && DENSITIES.has(v as AppearanceDensity)
		? (v as AppearanceDensity)
		: DEFAULT_APPEARANCE.density;
}

/**
 * Normalize enabled hot tabs. Accepts a boolean map or an id array.
 * Enforces at least one selected (falls back to Chat).
 */
export function normalizeTopNavbarTabs(
	input?: Partial<TopNavbarTabsPrefs> | TopNavbarTabId[] | null,
): TopNavbarTabsPrefs {
	const next: TopNavbarTabsPrefs = { ...DEFAULT_TOP_NAVBAR_TABS };

	if (Array.isArray(input)) {
		for (const id of TOP_NAVBAR_TAB_IDS) next[id] = false;
		for (const id of input) {
			if (typeof id === "string" && TAB_ID_SET.has(id as TopNavbarTabId)) {
				next[id as TopNavbarTabId] = true;
			}
		}
	} else if (input && typeof input === "object") {
		for (const id of TOP_NAVBAR_TAB_IDS) {
			const v = input[id];
			if (typeof v === "boolean") next[id] = v;
		}
	}

	if (!TOP_NAVBAR_TAB_IDS.some((id) => next[id])) {
		next.chat = true;
	}
	return next;
}

export function enabledTopNavbarTabs(
	prefs: Pick<AppearancePrefs, "topNavbarTabs"> | TopNavbarTabsPrefs,
): TopNavbarTabDef[] {
	const tabs =
		"topNavbarTabs" in prefs
			? normalizeTopNavbarTabs(prefs.topNavbarTabs)
			: normalizeTopNavbarTabs(prefs);
	return TOP_NAVBAR_TAB_DEFS.filter((t) => tabs[t.id]);
}

export function matchesTopNavbarTab(
	tab: TopNavbarTabDef,
	path: string,
): boolean {
	if (tab.id === "work") return path === "/";
	return path === tab.path || path.startsWith(`${tab.path}/`);
}

/** Hot-tab def for the current path, or null if not a hot-tab route. */
export function topNavbarTabForPath(path: string): TopNavbarTabDef | null {
	return TOP_NAVBAR_TAB_DEFS.find((t) => matchesTopNavbarTab(t, path)) ?? null;
}

export function firstVisibleTopNavbarPath(
	prefs: Pick<AppearancePrefs, "topNavbarTabs"> | TopNavbarTabsPrefs,
): string {
	return enabledTopNavbarTabs(prefs)[0]?.path ?? "/agent-chat";
}

/** Loose input from settings / cache — missing tab keys default to shown. */
export type AppearanceInput = {
	theme?: unknown;
	fontSize?: unknown;
	density?: unknown;
	topNavbarTabs?: Partial<TopNavbarTabsPrefs> | TopNavbarTabId[] | null;
};

export function normalizeAppearance(
	input?: AppearanceInput | null,
): AppearancePrefs {
	return {
		theme: asTheme(input?.theme),
		fontSize: asFontSize(input?.fontSize),
		density: asDensity(input?.density),
		topNavbarTabs: normalizeTopNavbarTabs(input?.topNavbarTabs),
	};
}

export function systemPrefersDark(
	media: { matches: boolean } | null | undefined = undefined,
): boolean {
	if (media) return media.matches;
	if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
		return true;
	}
	return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveThemeMode(
	theme: AppearanceTheme,
	prefersDark: boolean = systemPrefersDark(),
): "dark" | "light" {
	if (theme === "system") return prefersDark ? "dark" : "light";
	return theme;
}

export function cacheAppearance(prefs: AppearancePrefs): void {
	try {
		if (typeof localStorage === "undefined") return;
		localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify(prefs));
	} catch {
		/* restricted webviews / tests */
	}
}

export function readCachedAppearance(): AppearancePrefs | null {
	try {
		if (typeof localStorage === "undefined") return null;
		const raw = localStorage.getItem(APPEARANCE_STORAGE_KEY);
		if (!raw) return null;
		const parsed = JSON.parse(raw) as Partial<AppearancePrefs>;
		return normalizeAppearance(parsed);
	} catch {
		return null;
	}
}

export type AppearanceRoot = {
	classList: { toggle: (token: string, force?: boolean) => unknown };
	dataset: DOMStringMap | Record<string, string | undefined>;
	style: { colorScheme: string };
};

export function applyAppearance(
	input?: AppearanceInput | null,
	root: AppearanceRoot = document.documentElement,
	prefersDark: boolean = systemPrefersDark(),
): AppearancePrefs {
	const prefs = normalizeAppearance(input);
	const mode = resolveThemeMode(prefs.theme, prefersDark);
	root.classList.toggle("dark", mode === "dark");
	root.dataset.theme = mode;
	root.dataset.fontSize = prefs.fontSize;
	root.dataset.density = prefs.density;
	root.style.colorScheme = mode;
	cacheAppearance(prefs);
	return prefs;
}

type MediaQueryListLike = {
	matches: boolean;
	addEventListener?: (type: "change", listener: () => void) => void;
	removeEventListener?: (type: "change", listener: () => void) => void;
	addListener?: (listener: () => void) => void;
	removeListener?: (listener: () => void) => void;
};

let systemListenerCleanup: (() => void) | null = null;

/**
 * Re-apply when OS light/dark changes and the user chose System.
 * Pass a getter so each change reads the latest prefs.
 */
export function bindSystemThemeListener(
	getPrefs: () => AppearanceInput | null | undefined,
	root: AppearanceRoot = document.documentElement,
): () => void {
	systemListenerCleanup?.();
	systemListenerCleanup = null;

	if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
		return () => undefined;
	}

	const mq = window.matchMedia(
		"(prefers-color-scheme: dark)",
	) as unknown as MediaQueryListLike;

	const onChange = () => {
		const prefs = normalizeAppearance(getPrefs());
		if (prefs.theme !== "system") return;
		applyAppearance(prefs, root, mq.matches);
	};

	if (typeof mq.addEventListener === "function") {
		mq.addEventListener("change", onChange);
		systemListenerCleanup = () => mq.removeEventListener?.("change", onChange);
	} else if (typeof mq.addListener === "function") {
		mq.addListener(onChange);
		systemListenerCleanup = () => mq.removeListener?.(onChange);
	}

	return () => {
		systemListenerCleanup?.();
		systemListenerCleanup = null;
	};
}
