import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const BannerContainer = styled.div`
  background: linear-gradient(135deg, ${config.theme.colors.primary}15, ${config.theme.colors.accent}10);
  border: 1px solid ${config.theme.colors.border};
  border-radius: 12px;
  padding: 24px 20px;
  text-align: center;
  margin: 16px;
  backdrop-filter: blur(10px);
`;

export const BannerTitle = styled.h2`
  color: ${config.theme.colors.text};
  font-size: 18px;
  font-weight: 600;
  margin: 0 0 4px 0;
  letter-spacing: 0.5px;
`;

export const BannerSubtitle = styled.p`
  color: ${config.theme.colors.text};
  opacity: 0.8;
  font-size: 14px;
  margin: 0;
  font-weight: 400;
`;