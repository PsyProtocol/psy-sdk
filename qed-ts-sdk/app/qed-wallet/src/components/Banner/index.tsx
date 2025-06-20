import React from 'react';
import { useWalletConfig } from '../../config';
import { BannerContainer, BannerTitle, BannerSubtitle } from './Banner.styles';

interface BannerProps {
  title?: string;
  subtitle?: string;
}

export const Banner: React.FC<BannerProps> = ({ title, subtitle }) => {
  const { config } = useWalletConfig();
  
  return (
    <BannerContainer>
      <BannerTitle>{title || config.extension.title}</BannerTitle>
      {subtitle && <BannerSubtitle>{subtitle}</BannerSubtitle>}
    </BannerContainer>
  );
};

export default Banner;