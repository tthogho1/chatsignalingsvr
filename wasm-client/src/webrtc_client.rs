use js_sys::{Array, Object, Reflect};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    console, MediaStream, MediaStreamConstraints, MediaStreamTrack, RtcConfiguration,
    RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection, RtcPeerConnectionIceEvent,
    RtcPeerConnectionState, RtcSdpType, RtcSessionDescriptionInit, RtcTrackEvent,
};

pub struct WebRTCClient {
    peer_connection: RtcPeerConnection,
    local_stream: Rc<RefCell<Option<MediaStream>>>,
    is_camera_on: Rc<RefCell<bool>>,
    is_mic_on: Rc<RefCell<bool>>,
}

impl WebRTCClient {
    pub async fn new(
        ontrack_callback: &Closure<dyn FnMut(RtcTrackEvent)>,
        onicecandidate_callback: &Closure<dyn FnMut(RtcPeerConnectionIceEvent)>,
    ) -> Result<Self, JsValue> {
        // Create RTCConfiguration
        let config = RtcConfiguration::new();

        // Set ICE servers
        let ice_servers = Array::new();
        let stun_server1 = Object::new();
        Reflect::set(
            &stun_server1,
            &"urls".into(),
            &"stun:stun.l.google.com:19302".into(),
        )?;
        ice_servers.push(&stun_server1);

        let stun_server2 = Object::new();
        Reflect::set(
            &stun_server2,
            &"urls".into(),
            &"stun:stun1.l.google.com:19302".into(),
        )?;
        ice_servers.push(&stun_server2);

        config.set_ice_servers(&ice_servers);

        // Create peer connection
        let peer_connection = RtcPeerConnection::new_with_configuration(&config)?;

        let client = WebRTCClient {
            peer_connection,
            local_stream: Rc::new(RefCell::new(None)),
            is_camera_on: Rc::new(RefCell::new(true)),
            is_mic_on: Rc::new(RefCell::new(true)),
        };

        client.setup_peer_connection_handlers(ontrack_callback, onicecandidate_callback)?;

        Ok(client)
    }

    fn setup_peer_connection_handlers(
        &self,
        ontrack_callback: &Closure<dyn FnMut(RtcTrackEvent)>,
        onicecandidate_callback: &Closure<dyn FnMut(RtcPeerConnectionIceEvent)>,
    ) -> Result<(), JsValue> {
        self.peer_connection
            .set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
        self.peer_connection
            .set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));

        // OnConnectionStateChange handler
        let pc_clone = self.peer_connection.clone();
        let onconnectionstatechange_callback = Closure::wrap(Box::new(move |_event: JsValue| {
            let state: RtcPeerConnectionState = pc_clone.connection_state();
            let state_str = match state {
                RtcPeerConnectionState::New => "new",
                RtcPeerConnectionState::Connecting => "connecting",
                RtcPeerConnectionState::Connected => "connected",
                RtcPeerConnectionState::Disconnected => "disconnected",
                RtcPeerConnectionState::Failed => "failed",
                RtcPeerConnectionState::Closed => "closed",
                _ => "unknown",
            };
            console::log_2(
                &"WebRTC connection state changed:".into(),
                &state_str.into(),
            );
        }) as Box<dyn FnMut(JsValue)>);

        self.peer_connection.set_onconnectionstatechange(Some(
            onconnectionstatechange_callback.as_ref().unchecked_ref(),
        ));
        onconnectionstatechange_callback.forget();

        Ok(())
    }

    pub async fn get_user_media(&self) -> Result<MediaStream, JsValue> {
        let window = web_sys::window().ok_or("No global window exists")?;
        let navigator = window.navigator();
        let media_devices = navigator.media_devices()?;

        // Create constraints
        let constraints = MediaStreamConstraints::new();
        constraints.set_video(&true.into());
        constraints.set_audio(&true.into());

        // Get user media
        let promise = media_devices.get_user_media_with_constraints(&constraints)?;
        let stream = JsFuture::from(promise).await?;
        let media_stream: MediaStream = stream.dyn_into()?;

        // Store local stream
        *self.local_stream.borrow_mut() = Some(media_stream.clone());

        // Add tracks to peer connection
        let tracks = media_stream.get_tracks();
        console::log_2(
            &"Adding tracks to peer connection, count:".into(),
            &tracks.length().into(),
        );

        let streams_array = Array::new();
        streams_array.push(&media_stream);

        for i in 0..tracks.length() {
            let track = tracks.get(i).dyn_into::<MediaStreamTrack>()?;
            console::log_2(&"Adding track:".into(), &track.kind().into());
            self.peer_connection
                .add_track(&track, &media_stream, &streams_array);
        }

        console::log_1(&"All tracks added to peer connection".into());
        Ok(media_stream)
    }

    pub async fn create_offer(&self) -> Result<String, JsValue> {
        console::log_1(&"Creating WebRTC offer...".into());

        let promise = self.peer_connection.create_offer();
        let offer = JsFuture::from(promise).await?;

        let sdp_value = Reflect::get(&offer, &JsValue::from_str("sdp"))?;
        let sdp = sdp_value.as_string().unwrap_or_default();

        let sdp_type_value = Reflect::get(&offer, &JsValue::from_str("type"))?;
        let sdp_type_str = sdp_type_value.as_string().unwrap_or_else(|| "offer".to_string());

        let sdp_type = match sdp_type_str.as_str() {
            "answer" => RtcSdpType::Answer,
            "pranswer" => RtcSdpType::Pranswer,
            "rollback" => RtcSdpType::Rollback,
            _ => RtcSdpType::Offer,
        };

        let offer_init = RtcSessionDescriptionInit::new(sdp_type);
        offer_init.set_sdp(&sdp);

        let set_local_promise = self.peer_connection.set_local_description(&offer_init);
        JsFuture::from(set_local_promise).await?;

        Ok(sdp)
    }

    pub async fn handle_offer(&self, sdp: &str) -> Result<String, JsValue> {
        // Create remote description from offer
        let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        remote_desc.set_sdp(sdp);

        // Set remote description
        let set_remote_promise = self.peer_connection.set_remote_description(&remote_desc);
        JsFuture::from(set_remote_promise).await?;

        // Create answer
        let promise = self.peer_connection.create_answer();
        let answer = JsFuture::from(promise).await?;

        let sdp_value = Reflect::get(&answer, &JsValue::from_str("sdp"))?;
        let sdp = sdp_value.as_string().unwrap_or_default();

        let sdp_type_value = Reflect::get(&answer, &JsValue::from_str("type"))?;
        let sdp_type_str = sdp_type_value.as_string().unwrap_or_else(|| "answer".to_string());

        let sdp_type = match sdp_type_str.as_str() {
            "offer" => RtcSdpType::Offer,
            "pranswer" => RtcSdpType::Pranswer,
            "rollback" => RtcSdpType::Rollback,
            _ => RtcSdpType::Answer,
        };

        let answer_init = RtcSessionDescriptionInit::new(sdp_type);
        answer_init.set_sdp(&sdp);

        let set_local_promise = self.peer_connection.set_local_description(&answer_init);
        JsFuture::from(set_local_promise).await?;

        Ok(sdp)
    }

    pub async fn handle_answer(&self, sdp: &str) -> Result<(), JsValue> {
        // Create remote description from answer
        let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        remote_desc.set_sdp(sdp);

        // Set remote description
        let set_remote_promise = self.peer_connection.set_remote_description(&remote_desc);
        JsFuture::from(set_remote_promise).await?;

        Ok(())
    }

    pub async fn handle_ice_candidate(&self, candidate_str: &str) -> Result<(), JsValue> {
        console::log_1(&"Processing received ICE candidate".into());

        // Create ICE candidate from string
        let ice_candidate_init = RtcIceCandidateInit::new("");
        ice_candidate_init.set_candidate(candidate_str);
        let ice_candidate = RtcIceCandidate::new(&ice_candidate_init)?;

        // Add ICE candidate to peer connection
        let promise = self
            .peer_connection
            .add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice_candidate));
        JsFuture::from(promise).await?;

        console::log_1(&"ICE candidate added successfully".into());
        Ok(())
    }

    pub async fn toggle_camera(&self) -> Result<bool, JsValue> {
        let mut is_on = self.is_camera_on.borrow_mut();
        *is_on = !*is_on;
        if let Some(ref stream) = &*self.local_stream.borrow() {
            let video_tracks = stream.get_video_tracks();
            for i in 0..video_tracks.length() {
                if let Ok(track) = video_tracks.get(i).dyn_into::<MediaStreamTrack>() {
                    track.set_enabled(*is_on);
                }
            }
        }
        Ok(*is_on)
    }

    pub async fn toggle_microphone(&self) -> Result<bool, JsValue> {
        let mut is_on = self.is_mic_on.borrow_mut();
        *is_on = !*is_on;
        if let Some(ref stream) = &*self.local_stream.borrow() {
            let audio_tracks = stream.get_audio_tracks();
            for i in 0..audio_tracks.length() {
                if let Ok(track) = audio_tracks.get(i).dyn_into::<MediaStreamTrack>() {
                    track.set_enabled(*is_on);
                }
            }
        }
        Ok(*is_on)
    }

    pub fn close_connection(&self) -> Result<(), JsValue> {
        if self.peer_connection.connection_state() != RtcPeerConnectionState::Closed {
            self.peer_connection.close();
        }
        Ok(())
    }
}
