import { useCombobox, Combobox, InputBase, Input, Group, Text } from "@mantine/core";
import { WalletWidgetRPC } from "../../../../utils/rpc/walletRPC";
import React, { useEffect, useState } from "react";
import {TWalletAbility} from "doge-sdk"
import {IconMessage, IconBinary} from "@tabler/icons-react"
type TSigningAbilitiy = "sign-hash-sha256" | "sign-hash-raw";
interface ISigningAbilityOption {
  id: TSigningAbilitiy;
  label: string; 
  description: string;
  icon: any;
}
const SigningAbilities: ISigningAbilityOption[] = [
  {
    id: "sign-hash-sha256", 
    label: "Message (SHA256)", 
    description: "Sign the hash of an arbitrary message",
    icon: <IconMessage />
  },
  {
    id: "sign-hash-raw",
    label: "Raw Hash",
    description: "Sign a raw, 32-byte hash",
    icon: <IconBinary />
  },
];
function SelectOption({
  label,
  description,
  icon,
}: ISigningAbilityOption) {
  return (
    <Group>
      {icon}
      <div>
        <Text fz="sm" fw={500}>
          {label}
        </Text>
        <Text fz="xs" opacity={0.6}>
          {description}
        </Text>
      </div>
    </Group>
  );
}


function getSigningAbilities(abilities: TWalletAbility[]): TSigningAbilitiy[] {
  return SigningAbilities.filter((ability) => abilities.includes(ability.id)).map((ability) => ability.id);
}

interface ISignMessageSelectProps {
  abilities: TSigningAbilitiy[];
  value: TSigningAbilitiy;
  onChange: (value: TSigningAbilitiy) => void;
}

const SignMessageSelect = ({abilities, value, onChange}: ISignMessageSelectProps) => {
  
  const combobox = useCombobox({
    onDropdownClose: () => combobox.resetSelectedOption(),
  });

  const options = SigningAbilities.map((item) => (
    <Combobox.Option value={item.id} key={item.id}>
      <SelectOption {...item} />
    </Combobox.Option>
  ));
  const selectedOption = SigningAbilities.find((option) => option.id === value);

  return (
    <Combobox
      store={combobox}
      onOptionSubmit={(val) => {
        onChange(val as TSigningAbilitiy);
        combobox.closeDropdown();
      }}
    >
      <Combobox.Target>
        <InputBase
          component="button"
          type="button"
          pointer
          rightSection={<Combobox.Chevron />}
          onClick={() => combobox.toggleDropdown()}
          rightSectionPointerEvents="none"
          multiline
          label="Signature Message Type"
        >
          {selectedOption ? (
            <SelectOption {...selectedOption} />
          ) : (
            <Input.Placeholder>Select a Signature Message Type...</Input.Placeholder>
          )}
        </InputBase>
      </Combobox.Target>

      <Combobox.Dropdown>
        <Combobox.Options mah={200} style={{ overflowY: 'auto' }}>{options}</Combobox.Options>
      </Combobox.Dropdown>
    </Combobox>
  );
}

export {
  getSigningAbilities,
  SignMessageSelect,
}