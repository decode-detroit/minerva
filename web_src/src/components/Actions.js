import React from 'react';
import { stopPropogation } from './functions';
import { AddMenu } from './Menus';
import { SendNode } from './Nodes';

// An action list element
export class Action extends React.PureComponent {
  // Render the event action
  render() {
    // Switch based on the props
    if (this.props.action.hasOwnProperty(`NewScene`)) {
      return (
        <NewScene newScene={this.props.action.NewScene} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );
    
    // Modify Status
    } else if (this.props.action.hasOwnProperty(`ModifyStatus`)) {
      return (
        <ModifyStatus modifyStatus={this.props.action.ModifyStatus} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );
    
    // Cue Event
    } else if (this.props.action.hasOwnProperty(`CueEvent`)) {
      return (
        <CueEvent cueEvent={this.props.action.CueEvent} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );
    
    // Cancel Event
    } else if (this.props.action.hasOwnProperty(`CancelEvent`)) {
      return (
        <CancelEvent cancelEvent={this.props.action.CancelEvent} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );
    
    // Save Data
    } else if (this.props.action.hasOwnProperty(`SaveData`)) {
      return (
        <SaveData saveData={this.props.action.SaveData} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );
    
    // Send Data
    } else if (this.props.action.hasOwnProperty(`SendData`)) {
      return (
        <SendData sendData={this.props.action.SendData} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );

    // Select Event
    } else if (this.props.action.hasOwnProperty(`SelectEvent`)) {
      return (
        <SelectEvent selectEvent={this.props.action.SelectEvent} grabFocus={this.props.grabFocus} changeAction={this.props.changeAction}/>
      );
    }
    
    // Otherwise, return the default
    return (
        <div className="action">Invalid Action</div>
    );
  }
}

// A new scene action
export class NewScene extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      isMenuVisible: false,
      description: "Loading ...",
    }

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.newScene.new_scene.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        this.setState({
          description: json.item.itemPair.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the description of the scene
  componentDidMount() {
    this.updateItem();
  }

  // On change of item id, pull the description of the scene
  componentDidUpdate() {
    this.updateItem();
  }

  // Render the completed action
  render() {
    return (
      <>
        <ActionFragment title="New Scene" nodeType="scene" focusOn={() => this.props.grabFocus(this.props.newScene.new_scene.id)} changeAction={this.props.changeAction} content={
          <div className="actionDetail" onClick={(e) => {stopPropogation(e); this.setState(prevState => ({ isMenuVisible: !prevState.isMenuVisible }))}}>
            {this.state.description}
            <div className="editNote">Click To Change</div>
            {this.state.isMenuVisible && <AddMenu type="scene" left={200} top={100} addItem={(id) => {this.setState({ isMenuVisible: false }); this.props.changeAction({
              NewScene: {
                new_scene: {
                  id: id
                }
              }
            })}}/>}
          </div>
        }/>
      </>
    );
  }
}

// A modify status action
export class ModifyStatus extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      isStatusMenuVisible: false,
      isStateMenuVisible: false,
      description: "Loading ...",
      stateDescription: "Loading ...",
    }

    // The timeout to save changes, if set
    this.saveTimeout = null;

    // Bind the various functions
    this.updateItems = this.updateItems.bind(this);
  }

  // Helper function to update the item information
  async updateItems() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.modifyStatus.status_id.id}`);
      const json1 = await response.json();

      // Fetch the description of the state
      response = await fetch(`getItem/${this.props.modifyStatus.new_state.id}`);
      const json2 = await response.json();

      // If both are valid, save the result to the state
      if (json1.item.isValid && json2.item.isValid) {
        this.setState({
          description: json1.item.itemPair.description,
          stateDescription: json2.item.itemPair.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the description of the scene
  componentDidMount() {
    this.updateItems();
  }

  // On change of item id, pull the description of the scene
  componentDidUpdate(prevProps, prevState) {
    // Update the item descriptions, if either changed
    if ((this.props.modifyStatus.status_id.id !== prevProps.modifyStatus.status_id.id) || (this.props.modifyStatus.new_state.id !== prevProps.modifyStatus.new_state.id)) {
      this.updateItems();
    }
  }

  // Render the completed action
  render() {
    return (
      <>
        <ActionFragment title="Modify Status" nodeType="status" focusOn={() => this.props.grabFocus(this.props.modifyStatus.status_id.id)} changeAction={this.props.changeAction} content={
          <div className="actionDetail" onClick={stopPropogation}>
            <div onClick={() => {this.setState(prevState => ({ isStatusMenuVisible: !prevState.isStatusMenuVisible }))}}>{this.state.description}</div>
            <div className="editNote" onClick={() => {this.setState(prevState => ({ isStatusMenuVisible: !prevState.isStatusMenuVisible }))}}>Click To Change</div>
            <div className="additionalInfo">New State:
              <div className="additionalInfoDetail" onClick={() => {this.setState(prevState => ({ isStateMenuVisible: !prevState.isStateMenuVisible }))}}>{this.state.stateDescription}</div>
              <SendNode type="event" onMouseDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.modifyStatus.new_state.id)}}/>
            </div>
            {this.state.isStatusMenuVisible && <AddMenu type="status" left={200} top={100} addItem={(id) => {this.setState({ isStatusMenuVisible: false }); this.props.changeAction({
              ModifyStatus: {
                status_id: {
                  id: id,
                },
                new_state: this.props.modifyStatus.new_state,
              }
            })}}/>}
            {this.state.isStateMenuVisible && <AddMenu type="event" left={200} top={100} addItem={(id) => {this.setState({ isStateMenuVisible: false }); this.props.changeAction({
              ModifyStatus: {
                status_id: this.props.modifyStatus.status_id,
                new_state: {
                  id: id,
                },
              }
            })}}/>}
          </div>
        }/>
      </>
    );
  }
}

// A cue event action
export class CueEvent extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      isMenuVisible: false,
      description: "Loading ...",
      delay: this.props.cueEvent.event.delay ? (this.props.cueEvent.event.delay.secs + (this.props.cueEvent.event.delay.nanos / 1000000000)) : 0,
    }

    // The timeout to save changes, if set
    this.saveTimeout = null;

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handleChange = this.handleChange.bind(this);
    this.updateAction = this.updateAction.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.cueEvent.event.event_id.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        this.setState({
          description: json.item.itemPair.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Function to handle new delay in the input
  handleChange(e) {
    // Extract the value
    let value = e.target.value;

    // Replace the existing delay
    this.setState({
      delay: value,
    });

    // Clear the existing timeout, if it exists
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    // Set the new timeout
    this.saveTimeout = setTimeout(this.updateAction, 1000);
  }

  // Helper function to update the action
  updateAction(id) {
    // Use the default id if a new one not provided
    let new_id = id || this.props.cueEvent.event.event_id.id;

    // Clear the timeout, if it exists
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    // Update the action, with or without delay
    if (this.state.delay !== 0) {
      this.props.changeAction({
        CueEvent: {
          event: {
            event_id: {
              id: new_id
            },
            delay: {
              secs: parseInt(this.state.delay),
              nanos: (this.state.delay * 1000000000) % 1000000000, 
            },
          }
        }
      })
    } else {
      this.props.changeAction({
        CueEvent: {
          event: {
            event_id: {
              id: new_id
            }
          }
        }
      })
    }
  }


  // On initial load, pull the description of the scene
  componentDidMount() {
    this.updateItem();
  }

  // On change of item id, pull the description of the scene
  componentDidUpdate(prevProps, prevState) {
    // Update the item description, if it changed
    if (this.props.cueEvent.event.event_id.id !== prevProps.cueEvent.event.event_id.id) {
      this.updateItem();
    }
    
    // Update the event delay, if it changed
    if (this.props.cueEvent.delay && (this.props.cueEvent.event.delay.secs !== prevProps.cueEvent.event.delay.secs || this.props.cueEvent.event.delay.nanos !== prevProps.cueEvent.event.delay.nanos)) {
      this.setState({
        delay: this.props.cueEvent.event.delay.secs + (this.props.cueEvent.event.delay.nanos / 1000000000),
      })
    
    // Update the delay if it is now nothing
    } else if (!this.props.cueEvent.delay && prevProps.cueEvent.delay) {
      this.setState({
        delay: 0,
      })
    }
  }

  // Render the completed action
  render() {
    return (
      <>
        <ActionFragment title="Cue Event" nodeType="event" focusOn={() => this.props.grabFocus(this.props.cueEvent.event.event_id.id)} changeAction={this.props.changeAction} content={
          <div className="actionDetail" onClick={stopPropogation}>
            <div onClick={() => {this.setState(prevState => ({ isMenuVisible: !prevState.isMenuVisible }))}}>{this.state.description}</div>
            <div className="editNote" onClick={() => {this.setState(prevState => ({ isMenuVisible: !prevState.isMenuVisible }))}}>Click To Change</div>
            <div className="additionalInfo">Delay 
              <input type="number" min="0" value={this.state.delay} onInput={this.handleChange}></input> Seconds
            </div>
            {this.state.isMenuVisible && <AddMenu type="event" left={200} top={100} addItem={(id) => {this.setState({ isMenuVisible: false }); this.updateAction(id)}}/>}
          </div>
        }/>
      </>
    );
  }
}

// A cancel event action
export class CancelEvent extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      isMenuVisible: false,
      description: "Loading ...",
    }

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.cancelEvent.event.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        this.setState({
          description: json.item.itemPair.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the description of the scene
  componentDidMount() {
    this.updateItem();
  }

  // On change of item id, pull the description of the scene
  componentDidUpdate() {
    this.updateItem();
  }

  // Render the completed action
  render() {
    return (
      <>
        <ActionFragment title="Cancel Event" nodeType="event" focusOn={() => this.props.grabFocus(this.props.cancelEvent.event.id)} changeAction={this.props.changeAction} content={
          <div className="actionDetail" onClick={(e) => {stopPropogation(e); this.setState(prevState => ({ isMenuVisible: !prevState.isMenuVisible }))}}>
            {this.state.description}
            <div className="editNote">Click To Change</div>
            {this.state.isMenuVisible && <AddMenu type="event" left={200} top={100} addItem={(id) => {this.setState({ isMenuVisible: false }); this.props.changeAction({
              CancelEvent: {
                event: {
                  id: id
                }
              }
            })}}/>}
          </div>
        }/>
      </>
    );
  }
}

// A select event action
export class SelectEvent extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Select Event" nodeType="status" focusOn={() => this.props.grabFocus(this.props.selectEvent.status_id.id)} changeAction={this.props.changeAction} content={<div>{this.props.selectEvent.status_id.id}</div>}/>
    );
  }
}

// A save data action
export class SaveData extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Save Data" nodeType="event" focusOn={() => {}} changeAction={this.props.changeAction} content={<div>Not Yet Available</div>}/>
    );
  }
}

// A send data action
export class SendData extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Send Data" nodeType="event" focusOn={() => {}} changeAction={this.props.changeAction} content={<div>Not Yet Available</div>}/>
    );
  }
}

// An action edit area partial
export class ActionFragment extends React.PureComponent {  
  constructor(props) {
    // Collect props and set initial state
    super(props);

    // Default state
    this.state = {
      open: false,
    }
  }
  
  // Render the partial action
  render() {
    return (
      <div className="action" onClick={() => {this.setState(prevState => ({open: !prevState.open}))}}>
        <div className="deleteAction" onClick={(e) => {stopPropogation(e); this.props.changeAction()}}>X</div>
        {this.props.title}
        <div className="openStatus">
          {this.state.open ? 'v' : '<'}
        </div>
        <SendNode type={this.props.nodeType} onMouseDown={(e) => {stopPropogation(e); this.props.focusOn()}}/>
        {this.state.open && this.props.content}
      </div>
    );
  }
}
