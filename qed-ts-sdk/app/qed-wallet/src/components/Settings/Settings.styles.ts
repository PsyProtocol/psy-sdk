import styled from 'styled-components';

export const SettingsContainer = styled.div`
  background-color: #ffffff;
  min-height: 100vh;
  padding: 20px 0;
`;

export const SettingsTitle = styled.h1`
  color: #73e7ff;
  text-align: center;
  margin-bottom: 30px;
  font-size: 24px;
  font-weight: 600;
`;

export const BackButton = styled.button`
  background: none;
  border: none;
  color: #73e7ff;
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  width: 40px;
  height: 40px;
  
  &:hover {
    background-color: rgba(115, 231, 255, 0.1);
    transform: scale(1.05);
  }
`;

export const SettingsSection = styled.div`
  background-color: #ffffff;
  border: 1px solid #73e7ff;
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
  
  .mantine-TextInput-label,
  .mantine-ColorInput-label,
  .mantine-NumberInput-label,
  .mantine-Switch-label {
    color: #73e7ff;
    font-weight: 500;
  }
  
  .mantine-TextInput-input,
  .mantine-ColorInput-input,
  .mantine-NumberInput-input {
    border-color: #73e7ff;
    
    &:focus {
      border-color: #73e7ff;
    }
  }
  
  .mantine-Button-root {
    background-color: #73e7ff;
    border-color: #73e7ff;
    
    &:hover {
      background-color: #5bb8d1;
      border-color: #5bb8d1;
    }
  }
  
  .mantine-Button-outline {
    color: #73e7ff;
    background-color: transparent;
    border-color: #73e7ff;
    
    &:hover {
      background-color: rgba(115, 231, 255, 0.1);
    }
  }
  
  .mantine-Tabs-tab {
    color: #73e7ff;
    
    &[data-active] {
      color: #73e7ff;
      border-color: #73e7ff;
    }
  }
  
  .mantine-Switch-track {
    border-color: #73e7ff;
    
    &[data-checked] {
      background-color: #73e7ff;
      border-color: #73e7ff;
    }
  }
`;