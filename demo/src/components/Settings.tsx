import { healthApi } from '../services/api';
import { settingsService } from '../services/settingsService';
import { updateApiBaseUrl } from '../services/api';
import { useState } from 'react';

interface SettingsProps {
  onUrlChange: () => Promise<void>;
}

export default function Settings({ onUrlChange }: SettingsProps) {
  const [url, setUrl] = useState(settingsService.getAggregatorUrl());
  const [isTestingConnection, setIsTestingConnection] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);
  const [saveResult, setSaveResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  const defaultUrl = settingsService.getDefaultUrl();
  const isUsingCustomUrl = settingsService.isUsingCustomUrl();

  const handleTestConnection = async () => {
    if (!url.trim()) {
      setTestResult({ success: false, message: 'Please enter a URL' });
      return;
    }

    setIsTestingConnection(true);
    setTestResult(null);

    try {
      // Temporarily update axios base URL for testing
      const originalUrl = settingsService.getAggregatorUrl();
      updateApiBaseUrl(url);

      // Try to fetch health endpoint
      const health = await healthApi.getHealth();

      // Restore original URL after test
      updateApiBaseUrl(originalUrl);

      setTestResult({
        success: true,
        message: `Connected successfully! Version: ${health.version}, Status: ${health.status}`,
      });
    } catch (error) {
      // Restore original URL after failed test
      updateApiBaseUrl(settingsService.getAggregatorUrl());

      setTestResult({
        success: false,
        message: `Connection failed: ${(error as Error).message}`,
      });
    } finally {
      setIsTestingConnection(false);
    }
  };

  const handleSave = async () => {
    if (!url.trim()) {
      setSaveResult({ success: false, message: 'Please enter a URL' });
      return;
    }

    setIsSaving(true);
    setSaveResult(null);

    try {
      // Save to localStorage
      settingsService.setAggregatorUrl(url);

      // Update axios instance
      updateApiBaseUrl(url);

      // Refetch solver data
      await onUrlChange();

      setSaveResult({
        success: true,
        message: 'Settings saved successfully! Solver data has been refreshed.',
      });
    } catch (error) {
      setSaveResult({
        success: false,
        message: `Failed to save settings: ${(error as Error).message}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = async () => {
    setIsSaving(true);
    setSaveResult(null);
    setTestResult(null);

    try {
      // Reset to default
      settingsService.resetAggregatorUrl();
      const newUrl = settingsService.getAggregatorUrl();
      setUrl(newUrl);

      // Update axios instance
      updateApiBaseUrl(newUrl);

      // Refetch solver data
      await onUrlChange();

      setSaveResult({
        success: true,
        message: 'Settings reset to default! Solver data has been refreshed.',
      });
    } catch (error) {
      setSaveResult({
        success: false,
        message: `Failed to reset settings: ${(error as Error).message}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="max-w-3xl mx-auto">
      <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">
        Settings
      </h2>

      <div className="card space-y-6">
        {/* Aggregator URL Section */}
        <div>
          <h3 className="text-lg font-semibold mb-4 text-slate-900 dark:text-white">
            Aggregator URL
          </h3>

          {/* Current Status */}
          <div className="mb-4 p-3 bg-slate-100 dark:bg-slate-900 rounded-lg border border-slate-300 dark:border-slate-700">
            <div className="flex items-center justify-between">
              <span className="text-sm text-slate-600 dark:text-slate-400">
                Status:
              </span>
              <span className="text-sm font-medium text-slate-900 dark:text-white">
                {isUsingCustomUrl
                  ? '🔧 Using Custom URL'
                  : '✓ Using Default URL'}
              </span>
            </div>
            <div className="flex items-center justify-between mt-2">
              <span className="text-sm text-slate-600 dark:text-slate-400">
                Default URL:
              </span>
              <span className="text-xs font-mono text-slate-700 dark:text-slate-300">
                {defaultUrl}
              </span>
            </div>
          </div>

          {/* URL Input */}
          <div className="mb-4">
            <label className="label-text">Aggregator URL</label>
            <input
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="http://localhost:3000"
              className="input-field font-mono"
              disabled={isTestingConnection || isSaving}
            />
            <p className="text-xs text-slate-500 dark:text-slate-500 mt-1">
              Enter the base URL of your OIF Aggregator instance (without
              trailing slash)
            </p>
          </div>

          {/* Test Result */}
          {testResult && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                testResult.success
                  ? 'bg-green-100 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                  : 'bg-red-100 dark:bg-red-900/20 border-red-300 dark:border-red-700'
              }`}
            >
              <p
                className={`text-sm ${
                  testResult.success
                    ? 'text-green-700 dark:text-green-400'
                    : 'text-red-700 dark:text-red-400'
                }`}
              >
                {testResult.success ? '✓' : '✗'} {testResult.message}
              </p>
            </div>
          )}

          {/* Save Result */}
          {saveResult && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                saveResult.success
                  ? 'bg-green-100 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                  : 'bg-red-100 dark:bg-red-900/20 border-red-300 dark:border-red-700'
              }`}
            >
              <p
                className={`text-sm ${
                  saveResult.success
                    ? 'text-green-700 dark:text-green-400'
                    : 'text-red-700 dark:text-red-400'
                }`}
              >
                {saveResult.success ? '✓' : '✗'} {saveResult.message}
              </p>
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex flex-wrap gap-3">
            <button
              onClick={handleTestConnection}
              disabled={isTestingConnection || isSaving}
              className="btn-secondary"
            >
              {isTestingConnection ? (
                <>
                  <span className="animate-spin inline-block mr-2">⟳</span>
                  Testing...
                </>
              ) : (
                '🔍 Test Connection'
              )}
            </button>

            <button
              onClick={handleSave}
              disabled={isTestingConnection || isSaving}
              className="btn-primary"
            >
              {isSaving ? (
                <>
                  <span className="animate-spin inline-block mr-2">⟳</span>
                  Saving...
                </>
              ) : (
                '💾 Save'
              )}
            </button>

            <button
              onClick={handleReset}
              disabled={isTestingConnection || isSaving || !isUsingCustomUrl}
              className="btn-secondary"
            >
              🔄 Reset to Default
            </button>
          </div>

          {/* Info Box */}
          <div className="mt-6 p-4 bg-blue-100 dark:bg-blue-900/20 border border-blue-300 dark:border-blue-700 rounded-lg">
            <p className="text-sm text-blue-700 dark:text-blue-400">
              <strong>ℹ️ Note:</strong> Changing the aggregator URL will reload
              all solver and asset data from the new endpoint. Make sure the URL
              is correct before saving.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
