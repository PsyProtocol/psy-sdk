import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const OnboardingContainer = styled.div`
  height: 100%;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: ${config.theme.colors.background};
  padding: 20px;
`;

export const OnboardingContent = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 320px;
  width: 100%;
`;

export const OnboardingLogo = styled.img`
  width: 64px;
  height: 64px;
  margin-bottom: 20px;
`;

export const OnboardingTitle = styled.h1`
  color: ${config.theme.colors.text};
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 12px 0;
  letter-spacing: 0.5px;
`;

export const OnboardingSubtitle = styled.p`
  color: ${config.theme.colors.text};
  opacity: 0.8;
  font-size: 14px;
  margin: 0 0 24px 0;
  line-height: 1.5;
`;

export const ActionCard = styled.button`
  background: linear-gradient(135deg, ${config.theme.colors.primary}08, ${config.theme.colors.accent}05);
  border: 1px solid ${config.theme.colors.border};
  border-radius: 12px;
  padding: 20px 16px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  flex: 1;
  min-height: 120px;
  
  &:hover {
    border-color: ${config.theme.colors.primary};
    background: linear-gradient(135deg, ${config.theme.colors.primary}15, ${config.theme.colors.accent}08);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(115, 231, 255, 0.2);
  }
  
  &:active {
    transform: translateY(0);
  }
`;

export const ActionIcon = styled.div`
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: linear-gradient(135deg, ${config.theme.colors.primary}, ${config.theme.colors.accent});
  display: flex;
  align-items: center;
  justify-content: center;
  color: ${config.theme.colors.primaryText};
  margin-bottom: 12px;
  box-shadow: 0 2px 8px rgba(115, 231, 255, 0.3);
`;

export const ActionTitle = styled.h3`
  color: ${config.theme.colors.text};
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 6px 0;
`;

export const ActionDescription = styled.p`
  color: ${config.theme.colors.text};
  opacity: 0.7;
  font-size: 12px;
  margin: 0;
  line-height: 1.4;
`;