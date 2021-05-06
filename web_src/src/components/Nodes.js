import React from 'react';
import { stopPropogation } from './functions';
import { AddMenu } from './Menus';

// An action list element
export class Action extends React.PureComponent {
  constructor(props) {
    // Collect props and set initial state
    super(props);

    // Bind the various functions
    /*this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);*/
  }

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
        <ModifyStatus modifyStatus={this.props.action.ModifyStatus} grabFocus={this.props.grabFocus}/>
      );
    
    // Cue Event
    } else if (this.props.action.hasOwnProperty(`CueEvent`)) {
      return (
        <CueEvent cueEvent={this.props.action.CueEvent} grabFocus={this.props.grabFocus}/>
      );
    
    // Cancel Event
    } else if (this.props.action.hasOwnProperty(`CancelEvent`)) {
      return (
        <CancelEvent cancelEvent={this.props.action.CancelEvent} grabFocus={this.props.grabFocus}/>
      );
    
    // Save Data
    } else if (this.props.action.hasOwnProperty(`SaveData`)) {
      return (
        <div className="action">
          Save Data (not available)
        </div>
      );
    
    // Send Data
    } else if (this.props.action.hasOwnProperty(`SendData`)) {
      return (
        <div className="action">
          Send Data (not available)
        </div>
      );

    // Select Event
    } else if (this.props.action.hasOwnProperty(`SelectEvent`)) {
      return (
        <SelectEvent selectEvent={this.props.action.SelectEvent} grabFocus={this.props.grabFocus}/>
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

  // A function to change the item selected


  // On initial load, pull the description of the scene
  componentDidMount() {
    this.updateItem();
  }

  // Render the completed action
  render() {
    return (
      <>
        <ActionFragment title="New Scene" nodeType="scene" focusOn={() => this.props.grabFocus(this.props.newScene.new_scene.id)} content={
          <div className="actionDetail" onClick={(e) => {stopPropogation(e); this.setState({ isMenuVisible: true })}}>
            {this.state.description}
            <div className="editNote">Click To Change</div>
          </div>
        }/>
        {this.state.isMenuVisible && <AddMenu left={200} top={100} addItem={(id) => {this.setState({ isMenuVisible: false }); this.props.changeAction({
          NewScene: {
            newScene: {
              new_scene: {
                id: id
              }
            }
          }
        })}}/>}
      </>
    );
  }
}

// A modify status action
export class ModifyStatus extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Modify Status" nodeType="status" focusOn={() => this.props.grabFocus(this.props.modifyStatus.status_id.id)} content={<div>{this.props.modifyStatus.status_id.id}</div>}/>
    );
  }
}

// A cue event action
export class CueEvent extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Cue Event" nodeType="event" focusOn={() => this.props.grabFocus(this.props.cueEvent.event.event_id.id)} content={<div>{this.props.cueEvent.event.event_id.id}</div>}/>
    );
  }
}

// A cancel event action
export class CancelEvent extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Cancel Event" nodeType="event" focusOn={() => this.props.grabFocus(this.props.cancelEvent.event.id)} content={<div>{this.props.cancelEvent.event.id}</div>}/>
    );
  }
}

// A select event action
export class SelectEvent extends React.PureComponent {  
  // Render the completed action
  render() {
    return (
      <ActionFragment title="Select Event" nodeType="status" focusOn={() => this.props.grabFocus(this.props.selectEvent.status_id.id)} content={<div>{this.props.selectEvent.status_id.id}</div>}/>
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
        {this.props.title}
        <div className="openStatus">
          {this.state.open ? '-' : '+'}
        </div>
        <SendNode type={this.props.nodeType} onMouseDown={(e) => {stopPropogation(e); this.props.focusOn()}}/>
        {this.state.open && this.props.content}
      </div>
    );
  }
}

// A receive Node element
export class ReceiveNode extends React.PureComponent {  
  // Render the completed node
  render() {
    return (
      <div className={`node ${this.props.type}`} onMouseDown={this.props.onMouseDown}></div>
    );
  }
}

// A send Node element
export class SendNode extends React.PureComponent {
  // Render the completed node
  render() {
    return (
      <div className={`node ${this.props.type}`} onMouseDown={this.props.onMouseDown}></div>
    );
  }
}