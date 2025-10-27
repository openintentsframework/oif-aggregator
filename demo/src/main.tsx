import './index.css';

import App from './App.tsx';
import { StrictMode } from 'react';
import { ThemeProvider } from './contexts/ThemeContext';
import { createRoot } from 'react-dom/client';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <App />
    </ThemeProvider>
  </StrictMode>
);
