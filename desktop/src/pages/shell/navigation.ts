const routes = [
  'general',
  'account',
  'styles',
  'llm',
  'dictionary',
  'diagnostics',
  'about',
  'home',
  'stats',
  'subscription',
];
export type DesktopNavigation = { selected: string; lastSettings: string };
export function initialNavigation(route: string): DesktopNavigation {
  const selected = route === 'clipboard' ? 'general' : routes.includes(route) ? route : 'home';
  return { selected, lastSettings: ['home', 'stats'].includes(selected) ? 'general' : selected };
}
export function navigateDesktop(current: DesktopNavigation, route: string): DesktopNavigation {
  const selected = route === 'settings' ? current.lastSettings : route;
  if (!routes.includes(selected)) return current;
  return {
    selected,
    lastSettings: ['home', 'stats'].includes(selected) ? current.lastSettings : selected,
  };
}
