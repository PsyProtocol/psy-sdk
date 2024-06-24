import { useState, useEffect, useMemo, useCallback } from "react";
import { Command } from "cmdk";
import classNames from "classnames";
import styles from "./CommandBarModal.module.scss";
import { IDEContext } from "../../utils/ideContext";
import {
  EditorUIEventType,
  IDEMenuId,
  IEditorUICommandBarEvent,
  IEditorUIOpenProjectEvent,
} from "@qstudio/eventhubs";
import {
  ICommandBarMenu,
  ICommandBarOption,
  ICommandBarSelection,
  MenuOptionsType,
  SelectActionType,
  TMenuId,
} from "../../commands/types";
import { TIDEMenuGenerator, resolveMenuState } from "../../commands/registry";
import { StandardOption } from "./OptionRenderers/Standard";
import { KeyboardEvent } from "react";
interface ICommandBarModalProps {
  className?: string;
  ctx: IDEContext;
  menuGenerator: TIDEMenuGenerator;
}

const CommandBarModal: React.FC<ICommandBarModalProps> = ({
  className,
  menuGenerator,
  ctx,
}) => {
  const [originId, setOriginId] = useState("");
  const [menuState, setMenuState] = useState<ICommandBarMenu<IDEMenuId>[]>([]);
  const [inputValue, setInputValue] = useState("");

  const closeCommandBar = ()=>{
    setMenuState([]);
    ctx.projectManager.uiEventHub.notify({type: EditorUIEventType.CloseCommandBar, originId});
    setOriginId("");
  }
  // Toggle the menu when ⌘K is pressed
  useEffect(() => {
    let isOpen = false;
    const onCommandBar = (e: IEditorUICommandBarEvent) => {
      if (menuState.length) {
        closeCommandBar();
        return;
      }
      if (e.defaultValue) {
        setInputValue(e.defaultValue);
      }
      setOriginId(e.originId||"");
      const menuStateNew = resolveMenuState(menuGenerator, e.menuType, {
        ctx,
        searchText: e.defaultValue || "",
        state: [],
        selectedOption: null as any,
      });

      setMenuState(menuStateNew);
    };

    const onChangeProject = (e: IEditorUIOpenProjectEvent) => {
      closeCommandBar();
    };

    ctx.projectManager.uiEventHub.on(
      EditorUIEventType.CommandBar,
      onCommandBar
    );
    ctx.projectManager.uiEventHub.on(
      EditorUIEventType.OpenProject,
      onChangeProject
    );

    return () => {
      ctx.projectManager.uiEventHub.remove(
        EditorUIEventType.CommandBar,
        onCommandBar
      );
      ctx.projectManager.uiEventHub.remove(
        EditorUIEventType.OpenProject,
        onChangeProject
      );
    };
  }, [ctx, menuGenerator, menuState]);
  const activeMenu: ICommandBarMenu<IDEMenuId> | null =
    menuState.length !== 0 ? menuState[menuState.length - 1] : null;

  const optionGroups = useMemo(() => {
    const am: ICommandBarMenu<IDEMenuId> | null =
      menuState.length !== 0 ? menuState[menuState.length - 1] : null;

    if (am) {
      const selectionState: ICommandBarSelection<IDEMenuId> = {
        ctx,
        searchText: inputValue,
        state: menuState,
        selectedOption: am,
      };
      const optionGroups =
        am.optionsType === MenuOptionsType.Dynamic
          ? am.optionGroupsGenerator(selectionState)
          : am.optionGroups;

      return optionGroups;
    } else {
      return [];
    }
  }, [menuState]);

  const onSelectOption = useCallback(
    (option: ICommandBarOption<IDEMenuId>) => {
      const selectionState: ICommandBarSelection<IDEMenuId> = {
        ctx,
        searchText: inputValue,
        state: menuState,
        selectedOption: option,
      };
      if (option.actionType === SelectActionType.NavigatePop) {
        setMenuState(
          menuState.slice(0, menuState.length - (option.popCount || 1))
        );
      } else if (option.actionType === SelectActionType.NavigateDepth) {
        setMenuState(menuState.slice(0, option.depth));
      } else if (option.actionType === SelectActionType.NavigateReplace) {
        setMenuState(
          resolveMenuState(menuGenerator, option.state, selectionState)
        );
      }else if(option.actionType === SelectActionType.NavigatePush){
        setMenuState([...menuState,...resolveMenuState(menuGenerator, option.subMenu, selectionState)])
      }else if(option.actionType === SelectActionType.NavigateHandler){
        option.handler(selectionState, (state)=>setMenuState(state));
      }else if(option.actionType === SelectActionType.CustomAction){
        const result = resolveMenuState(menuGenerator, option.handler(selectionState), selectionState);
        setMenuState(result);
      }else if(option.actionType === SelectActionType.CustomActionAsync){
        option.handler(selectionState).then((x)=>{
          const result = resolveMenuState(menuGenerator, x, selectionState);
          setMenuState(result);
        }).catch((e)=>{
          console.error("custom action error", e);
        })
      }
    },
    [menuState, menuGenerator, ctx, inputValue]
  );

  const isOpen = !!activeMenu;
  if (!isOpen) {
    return <div style={{ display: "none" }}></div>;
  }

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closeCommandBar();
    }
  };

  return (
    <div className={classNames(styles.commandBarModal, styles.dark)}>
      <div className={styles.overlay} onClick={() => closeCommandBar()}></div>
      <Command label="Command Menu" shouldFilter={!activeMenu.disableFilter}>
        <Command.Input
          onValueChange={(v) => setInputValue(v)}
          value={inputValue}
          placeholder={activeMenu?.placeholder}
          onKeyDown={onKeyDown}
          autoFocus
        />
        <Command.List>
          <Command.Empty>No results found.</Command.Empty>
          {optionGroups
            .filter((x) => x.options.length)
            .map((group) => {
              return (
                <Command.Group heading={group.label} key={group.id}>
                  {group.options.map((option) => {
                    return (
                      <StandardOption
                        key={option.id}
                        option={option}
                        onSelect={onSelectOption}
                      />
                    );
                  })}
                </Command.Group>
              );
            })}
        </Command.List>
      </Command>
    </div>
  );
};

export { CommandBarModal };
