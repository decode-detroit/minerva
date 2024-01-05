import React from 'react';
import { Action } from './Actions';
import { ReceiveNode } from './Nodes';
import { State } from './States';
import { stopPropogation } from './Functions';
import { AddMenu, AddActionMenu, DeleteMenu } from './Menus';

// An item box to select the appropriate sub-box
export class ItemBox extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      left: 0, // horizontal offset of the box
      top: 0, // vertical offest of the box
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      description: "Loading ...", // placeholder for the real data
      display: {},
      type: "",
      isDeleteVisible: false,
    };

    // The timeout to save changes, if set
    this.saveTimeout = null;

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handlePointerDown = this.handlePointerDown.bind(this);
    this.handlePointerMove = this.handlePointerMove.bind(this);
    this.handlePointerClose = this.handlePointerClose.bind(this);
    this.handleChange = this.handleChange.bind(this);
  }
  
  // Function to respond to clicking the area
  handlePointerDown(e) {
    stopPropogation(e);
   
    // Connect the pointer event handlers to the document
    document.onpointermove = this.handlePointerMove;
    document.onpointerup = this.handlePointerClose;

    // Get the current location
    let left = e.target.parentNode.offsetLeft;
    let top = e.target.parentNode.offsetTop;

    // Save the cursor position, hide the menu
    this.setState({
      left: left,
      top: top,
      cursorX: e.clientX,
      cursorY: e.clientY,
    });
  }

  // Function to respond to dragging the area
  handlePointerMove(e) {
    stopPropogation(e);

    // Update the state
    this.setState((state) => {
      // Calculate change from old cursor position
      let changeX = state.cursorX - e.clientX;
      let changeY = state.cursorY - e.clientY;
  
      // Calculate the new location
      let left = state.left - parseInt(changeX / this.props.zoom);
      let top = state.top - parseInt(changeY / this.props.zoom);
  
      // Enforce bounds on the new location
      left = (left >= 0) ? left : 0;
      top = (top >= 0) ? top : 0;
  
      // Save the new location and current cursor position
      return {
        left: left,
        top: top,
        cursorX: e.clientX,
        cursorY: e.clientY,
      }
    });
  }
  
  // Function to respond to releasing the pointer
  handlePointerClose() {
    // Stop moving when pointer button is released
    document.onpointermove = null;
    document.onpointerup = null;

    // Save the updated location
    this.props.saveLocation(this.props.id, this.state.left, this.state.top);

    // Clear the left and top state
    this.setState({left: 0, top: 0});
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        // Save the itemPair
        this.setState({
          description: json.data.item.description,
          display: json.data.item.display,
        });
      }

      // Check to see the item type
      response = await fetch(`getType/${this.props.id}`);
      const json2 = await response.json();

      // If valid, save the result to the state
      if (json2.isValid) {
        this.setState({
          type: json2.data.message,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the location and item information
  componentDidMount() {
    // Pull the item information
    this.updateItem();
  }

  // Function to handle new text in the input
  handleChange(e) {
    // Extract the value
    let value = e.target.value;

    // Replace the existing description
    this.setState({
      description: value,
    });

    // CLear the existing timeout, if it exists
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    // Save the changes after a second pause
    let modifications = [{
      modifyItem: {
        itemPair: { id: this.props.id, description: value, display: this.state.display },
      },
    }];
    this.saveTimeout = setTimeout(() => {
      this.props.saveModifications(modifications);
    }, 1000);
  }
 
  // Return the selected box
  render() {
    // Calculate the left and top offsets
    let left = this.state.left ? `${this.state.left}px` : ``;
    let top = this.state.top ? `${this.state.top}px` : ``;

    // Calculate isFocus
    let isFocus = this.props.focusId === this.props.id;

    // Return the item box
    return (
      <>
        {this.state.type !== "" &&
          <div id={`id-${this.props.id}`} className={`box ${this.state.type} row${this.props.row} ${isFocus ? 'focus' : ''}`} style={{ left: left, top: top }} onPointerDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.id)}}>
            <div className="title">
              <input type="text" value={this.state.description} size={this.state.description.length > 30 ? this.state.description.length - 10 : 20} onInput={this.handleChange}></input>
              <div className="itemId disableSelect">({this.props.id})</div>
              {isFocus && <div className="removeMenu disableSelect">
                <div onPointerDown={(e) => {stopPropogation(e); this.props.removeItem(this.props.id)}}>{this.props.removeText}</div>
                <div onPointerDown={(e) => {stopPropogation(e); this.setState({ isDeleteVisible: true })}}>Delete</div>
              </div>}
              {this.state.isDeleteVisible && <DeleteMenu id={this.props.id} afterDelete={() => this.props.removeItem(this.props.id)} closeMenu={() => {this.setState({ isDeleteVisible: false })}} saveModifications={this.props.saveModifications} />}
            </div>
            <ReceiveNode id={`receive-node-${this.props.id}`} type={this.state.type} onPointerDown={this.handlePointerDown}/>
            {isFocus && this.state.type === "scene" && <SceneFragment id={this.props.id} changeScene={this.props.changeScene} saveModifications={this.props.saveModifications}/>}
            {this.state.type === "group" && <GroupFragment id={this.props.id} focusId={this.props.focusId} zoom={this.props.zoom} grabFocus={this.props.grabFocus} changeScene={this.props.changeScene} saveModifications={this.props.saveModifications} saveLocation={this.props.saveLocation} saveDimensions={this.props.saveDimensions} createConnector={this.props.createConnector}/>}
            {isFocus && this.state.type === "status" && <StatusFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector} saveModifications={this.props.saveModifications}/>}
            {isFocus && this.state.type === "event" && <EventFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector} saveModifications={this.props.saveModifications}/>}
            {isFocus && this.state.type === "none" && <BlankFragment id={this.props.id} updateItem={this.updateItem} saveModifications={this.props.saveModifications}/>}
          </div>
        }
      </>
    );
  }
}

// An empty box with no type
export class BlankFragment extends React.PureComponent {
  // Return the fragment
  render() {
    return (
      <>
        <div className="subtitle">Choose Item Type</div>
        <div className="typeChooser">
          <div className="divButton event" onClick={() => {let modifications = [{ modifyEvent: { itemId: { id: this.props.id }, event: { actions: [] }}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Event</div>
          <div className="divButton status" onClick={() => {let modifications = [{ modifyStatus: { itemId: { id: this.props.id }, status: { MultiState: { current: { id: 0 }, allowed: [], no_change_silent: false, }}}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Status</div>
          <div className="divButton scene" onClick={() => {let modifications = [{ modifyScene: { itemId: { id: this.props.id }, scene: { items: [], groups: [], }}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Scene</div>
          <div className="divButton group" onClick={() => {let modifications = [{ modifyGroup: { itemId: { id: this.props.id }, group: { items: [], isHidden: true }}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Group</div>
        </div>
      </>
    );
  }
}

// A scene box with an scene and scene detail
export class SceneFragment extends React.PureComponent {
  // Return the fragment
  render() {
    return (
      <>
        <div className="divButton" onClick={() => {this.props.changeScene(this.props.id)}}>View This Scene</div>
        <EventFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector} saveModifications={this.props.saveModifications}/>
      </>
    );
  }
}

// A group box with other boxes inside
export class GroupFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      idList: [],
      isHidden: true,
      width: 0,
      height: 0,
      cursorX: 0,
      cursorY: 0,
      addLocX: 0,
      addLocY: 0,
      isMenuVisible: false,
    }

    // Bind the various functions
    this.handlePointerDown = this.handlePointerDown.bind(this);
    this.handlePointerMove = this.handlePointerMove.bind(this);
    this.handlePointerClose = this.handlePointerClose.bind(this);
    this.showContextMenu = this.showContextMenu.bind(this);
    this.addItemToGroup = this.addItemToGroup.bind(this);
    this.removeItemFromGroup = this.removeItemFromGroup.bind(this);
    this.updateGroup = this.updateGroup.bind(this);
    this.toggleShow = this.toggleShow.bind(this);
  }
  
  // Function to respond to clicking the area
  handlePointerDown(e) {
    stopPropogation(e);
   
    // Connect the pointer event handlers to the document
    document.onpointermove = this.handlePointerMove;
    document.onpointerup = this.handlePointerClose;

    // Get the current size
    let width = e.target.parentNode.querySelector('div[class="groupArea"]').offsetWidth;
    let height = e.target.parentNode.querySelector('div[class="groupArea"]').offsetHeight;

    // Save the size and cursor position
    this.setState({
      width: width,
      height: height,
      cursorX: e.clientX,
      cursorY: e.clientY,
    });
  }

  // Function to respond to dragging the area
  handlePointerMove(e) {
    stopPropogation(e);

    // Update the state
    this.setState((state) => {
      // Calculate change from old cursor position
      let changeX = state.cursorX - e.clientX;
      let changeY = state.cursorY - e.clientY;
  
      // Calculate the new dimensions
      let width = state.width - parseInt(changeX / this.props.zoom);
      let height = state.height - parseInt(changeY / this.props.zoom);
  
      // Enforce bounds on the new dimensions
      width = (width >= 250) ? width : 250;
      height = (height >= 100) ? height : 100;
  
      // Save the new dimensions and current cursor position
      return {
        width: width,
        height: height,
        cursorX: e.clientX,
        cursorY: e.clientY,
      }
    });
  }
  
  // Function to respond to releasing the pointer
  handlePointerClose() {
    // Stop moving when pointer button is released
    document.onpointermove = null;
    document.onpointerup = null;

    // Save the updated dimensions
    this.props.saveDimensions(this.props.id, this.state.width, this.state.height);

    // Clear the left and top state
    this.setState({width: 0, height: 0});
  }

  // Function to show the context menu at the correct location
  showContextMenu(e) {
    stopPropogation(e);
    e.preventDefault(); // block the browser menu

    // Update the cusor location and mark the menu as visible
    let rect = e.target.parentNode.getBoundingClientRect();
    this.setState({
      addLocX: e.clientX - rect.left,
      addLocY: e.clientY - rect.top,
      isMenuVisible: true,
    });
  }

  // Function to hide the menu and add an item to the group
  addItemToGroup(id, left, top) {
    // Add the item to the group, if missing
    this.setState((prevState) => {
      // If the id is not present
      if (!prevState.idList.includes(id)) {        
        // Add it to the list
        const newList = [...prevState.idList, id];

        // Convert the item list to item ids
        let items = newList.map((id) => {return { id: id }});

        // Compose the new group
        let newGroup = { items: items, isHidden: prevState.isHidden };

        // Save the changes
        console.log(newGroup);
        let modifications = [{
          modifyGroup: {
            itemId: { id: this.props.id },
            group: newGroup,
          },
        }];
        this.props.saveModifications(modifications);

        // Save the location
        this.props.saveLocation(id, parseInt((left / this.props.zoom)), parseInt((top / this.props.zoom)));
        
        // Update the state
        return ({
          idList: newList,
          isMenuVisible: false,
        });
      }

      // Otherwise, just close the add menu
      return ({
        isMenuVisible: false,
      });
    });
  }

  // Function to remove an item from the group
  removeItemFromGroup(id) {
    // Remove the item from the list, if present
    this.setState(prevState => {
      // Remove the id if present in the id list
      if (prevState.idList.includes(id)) {
        // Filter from the lists
        const newIdList = prevState.idList.filter(listId => listId !== id);

        // Convert list to item ids
        let items = newIdList.map((id) => {return { id: id }});

        // Compose the new group
        let newGroup = { items: items, isHidden: prevState.isHidden};

        // Save the changes
        let modifications = [{
          modifyGroup: {
            itemId: { id: this.props.id },
            group: newGroup,
          },
        }];
        this.props.saveModifications(modifications);
        
        // Update the state
        return ({
          idList: newIdList,
        });
      }

      // Otherwise, do nothing
      return ({});
    });
  }

  // Helper function to update the group information
  async updateGroup() {
    try {
      // Fetch the detail of the status
      const response = await fetch(`getGroup/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        // Strip ids from the items
        let ids = json.data.group.items.map((item) => item.id);

        // Exclude this id to prevent recursion
        let clean_ids = ids.filter((id) => id !== this.props.id)

        // Sort the ids and save        
        clean_ids.sort();
        this.setState({
          isHidden: json.data.group.isHidden,
          idList: clean_ids,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to hide or show the group items
  toggleShow() {
    // Toggle the hidden state
    this.setState((prevState) => {
      // Recompose the item ids
      let items = prevState.idList.map((id) => {return { id : id };});

      // Compose the new group
      let newGroup = { items: items, isHidden: !prevState.isHidden};

      // Save the changes
      let modifications = [{
        modifyGroup: {
          itemId: { id: this.props.id },
          group: newGroup,
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        isHidden: !prevState.isHidden,
      };
    });
  }

  // On initial load, pull the group information
  componentDidMount() {
    // Pull the new group information
    this.updateGroup();
  }

  // Return the fragment
  render() {
    // Calculate the width and height
    let width = this.state.width ? `${this.state.width}px` : ``;
    let height = this.state.height ? `${this.state.height}px` : ``;

    // Create a box for items that are in this group
    const boxes = this.state.idList.map((id, index) => <ItemBox key={id.toString()} id={id} focusId={this.props.focusId} row={parseInt(index / 12)} zoom={this.props.zoom} grabFocus={this.props.grabFocus} changeScene={this.props.changeScene} saveModifications={this.props.saveModifications} saveLocation={this.props.saveLocation} saveDimensions={this.props.saveDimensions} removeItem={this.removeItemFromGroup} removeText="Remove From Group" createConnector={this.props.createConnector}/>);
    
    // Return the group box
    return (
      <>
        {!this.state.isHidden && <div className="placeholder disableSelect">Right Click To Add Items</div>}
        {!this.state.isHidden && <div className="groupArea" style={{ width: width, height: height }} onContextMenu={this.showContextMenu}>
          {boxes}
        </div>}
        <div className="showCorner disableSelect" onPointerDown={(e) => {stopPropogation(e); this.toggleShow()}}>{this.state.isHidden ? "+Show+" : "-Hide-"}</div>
        {!this.state.isHidden && <div className="resizeCorner disableSelect" onPointerDown={this.handlePointerDown}>//</div>}
        {this.state.isMenuVisible && <AddMenu left={this.state.addLocX} top={this.state.addLocY} closeMenu={() => this.setState({ isMenuVisible: false })} addItem={this.addItemToGroup} saveModifications={this.props.saveModifications}/>}
      </>
    );
  }
}


// A statue box with a status and status detail
export class StatusFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      status: {},
      isMenuVisible: false,
    }

    // Bind the various functions
    this.updateStatus = this.updateStatus.bind(this);
    this.addState = this.addState.bind(this);
    this.removeState = this.removeState.bind(this);
  }

  // Helper function to update the status information
  async updateStatus() {
    try {
      // Fetch the detail of the status
      const response = await fetch(`getStatus/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        this.setState({
          status: json.data.status,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to add a new state
  addState(id) {
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      // Save the new state id
      this.setState((prevState) => {
        // Compose the new status
        let newStatus = { MultiState:
          {...prevState.status.MultiState, allowed: [...prevState.status.MultiState.allowed, {id: id}]}
        };

        // Save the changes
        let modifications = [{
          modifyStatus: {
            itemId: { id: this.props.id },
            status: newStatus,
          },
        }];
        this.props.saveModifications(modifications);

        // Update the local state and close the menu
        return {
          status: newStatus,
          isMenuVisible: false,
        };
      });
    
    // Ignore types other than multistate
    } else {
      console.error("This feature not yet implemented.");
    }
  }

  // Helper function to remove a state
  removeState(index) {
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      // Save the new state id
      this.setState((prevState) => {
        // Remove the state from the allowed list
        let newAllowed = [...prevState.status.MultiState.allowed];
        newAllowed.splice(index, 1);

        // Compose the new status
        let newStatus = { MultiState:
          {...prevState.status.MultiState, allowed: newAllowed}
        };

        // Save the changes
        let modifications = [{
          modifyStatus: {
            itemId: { id: this.props.id },
            status: newStatus,
          },
        }];
        this.props.saveModifications(modifications);

        // Update the local state
        return {
          status: newStatus,
        };
      });
    
    // Ignore types other than multistate
    } else {
      console.error("This feature not yet implemented.");
    }
  }

  // On initial load, pull the status information
  componentDidMount() {
    // Pull the new status information
    this.updateStatus();
  }

  // Return the fragment
  render() {
    // Compose any states into a list
    let children = [];
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      children = this.state.status.MultiState.allowed.map((state, index) => <State key={state.toString()} state={state} grabFocus={this.props.grabFocus} removeState={() => {this.removeState(index)}} createConnector={this.props.createConnector} />);
    }

    // Return the fragment
    return (
      <>
        <div className="subtitle">States:</div>
        <div className="verticalList" onWheel={stopPropogation}>{children}
          {this.state.selectMenu}
        </div>
        <div className="addButton" onClick={() => {this.setState(prevState => ({ isMenuVisible: !prevState.isMenuVisible }))}}>
          {this.state.isMenuVisible ? `-` : `+`}
          {this.state.isMenuVisible && <AddMenu type="event" left={180} top={60} closeMenu={() => this.setState({ isMenuVisible: false })} addItem={this.addState} saveModifications={this.props.saveModifications}/>}
        </div>
      </>
    );
  }
}

// An event box with an event and event detail
export class EventFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      eventActions: [], // placeholder for the read data
      isMenuVisible: false,
      selectMenu: null,
    }

    // Bind the various functions
    this.updateEvent = this.updateEvent.bind(this);
    this.addAction = this.addAction.bind(this);
    this.changeAction = this.changeAction.bind(this);
    this.selectMenu = this.selectMenu.bind(this);
  }

  // Helper function to update the event information
  async updateEvent() {
    try {
      // Fetch the detail of the event
      const response = await fetch(`getEvent/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state

      if (json.isValid) {
        this.setState({
          eventActions: json.data.event.actions,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to add a new event action
  addAction(action) {
    // Save the change action
    this.setState((prevState) => {
      // Copy the existing action list and add a blank action
      let newActions = [...prevState.eventActions, action];

      // Save the changes
      let modifications = [{
        modifyEvent: {
          itemId: { id: this.props.id },
          event: {
            actions: newActions,
          }
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        eventActions: [...newActions],
        isMenuVisible: false,
      };
    });
  }

  // Helper function to change an event action
  changeAction(index, action) {
    // Save the change action
    this.setState((prevState) => {
      // Copy the existing action list
      let newActions = [...prevState.eventActions];

      // If an action was specified, replace it
      if (action) {
        newActions[index] = action;

      // Otherwise, remove that index number
      } else {
        newActions.splice(index, 1);
      }

      // Save the changes
      let modifications = [{
        modifyEvent: {
          itemId: { id: this.props.id },
          event: {
            actions: newActions,
          }
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        eventActions: [...newActions],
      };
    });
  }

  // Helper function to save the new select menu
  selectMenu(newMenu) {
    // Check to see if it's already set
    if (newMenu !== null && this.state.selectMenu !== null) {
      return false;
    }
    
    // Otherwise, update it and return true
    this.setState({
      selectMenu: newMenu,
    })
    return true;
  }

  // On initial load, pull the event information
  componentDidMount() {
    // Pull the new event information
    this.updateEvent();
  }

  // Return the fragment
  render() {
    // Compose any actions into a list
    const children = this.state.eventActions.map((action, index) => <Action key={action.toString()} action={action} grabFocus={this.props.grabFocus} changeAction={(newAction) => {this.changeAction(index, newAction)}} selectMenu={this.selectMenu} createConnector={this.props.createConnector}/>);

    // Return the fragment
    return (
      <>
        <div className="subtitle">Actions:</div>
        <div className="verticalList" onWheel={stopPropogation}>
          {children}
          {this.state.selectMenu}
        </div>
        <div className="addButton" onClick={() => {this.setState(prevState => ({ isMenuVisible: !prevState.isMenuVisible }))}}>
          {this.state.isMenuVisible ? `-` : `+`}
          {this.state.isMenuVisible && <AddActionMenu left={180} top={60} addAction={this.addAction}/>}
        </div>
      </>
    );
  }
}

