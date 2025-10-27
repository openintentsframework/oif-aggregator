const AGGREGATOR_URL_KEY = 'oif-aggregator-url';
const DEFAULT_URL =
  import.meta.env.VITE_API_BASE_URL || 'http://localhost:4000';

class SettingsService {
  getAggregatorUrl(): string {
    try {
      const customUrl = localStorage.getItem(AGGREGATOR_URL_KEY);
      return customUrl || DEFAULT_URL;
    } catch (error) {
      console.error('Failed to get aggregator URL from localStorage:', error);
      return DEFAULT_URL;
    }
  }

  setAggregatorUrl(url: string): void {
    try {
      // Remove trailing slash for consistency
      const normalizedUrl = url.replace(/\/$/, '');
      localStorage.setItem(AGGREGATOR_URL_KEY, normalizedUrl);
    } catch (error) {
      console.error('Failed to save aggregator URL to localStorage:', error);
      throw new Error('Failed to save aggregator URL');
    }
  }

  resetAggregatorUrl(): void {
    try {
      localStorage.removeItem(AGGREGATOR_URL_KEY);
    } catch (error) {
      console.error('Failed to reset aggregator URL:', error);
      throw new Error('Failed to reset aggregator URL');
    }
  }

  getDefaultUrl(): string {
    return DEFAULT_URL;
  }

  isUsingCustomUrl(): boolean {
    try {
      return localStorage.getItem(AGGREGATOR_URL_KEY) !== null;
    } catch (error) {
      return false;
    }
  }
}

export const settingsService = new SettingsService();
