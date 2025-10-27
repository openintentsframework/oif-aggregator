import { RainbowKitProvider } from '@rainbow-me/rainbowkit';
import { useTheme } from '../contexts/ThemeContext';
import { lightThemeConfig, darkThemeConfig } from '../config/wallet';

interface RainbowKitWrapperProps {
  children: React.ReactNode;
}

export default function RainbowKitWrapper({ children }: RainbowKitWrapperProps) {
  const { theme } = useTheme();
  
  return (
    <RainbowKitProvider
      theme={theme === 'dark' ? darkThemeConfig : lightThemeConfig}
    >
      {children}
    </RainbowKitProvider>
  );
}
