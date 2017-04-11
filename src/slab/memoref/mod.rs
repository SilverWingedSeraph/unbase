pub mod serde;
use super::*;
use memorefhead::MemoRefHead;
use error::RetrieveError;

use std::sync::{Arc,RwLock};
use std::fmt;


#[derive(Clone)]
pub struct MemoRef(pub Arc<MemoRefInner>);

impl Deref for MemoRef {
    type Target = MemoRefInner;
    fn deref(&self) -> &MemoRefInner {
        &*self.0
    }
}

pub struct MemoRefInner {
    pub id:       MemoId,
    pub owning_slab_id: SlabId,
    pub subject_id: Option<SubjectId>,
    pub peerlist: RwLock<MemoPeerList>,
    pub ptr:      RwLock<MemoRefPtr>
}

pub enum MemoRefPtr {
    Resident(Memo),
    Remote
}

impl MemoRefPtr {
    pub fn to_peering_status (&self) -> MemoPeeringStatus {
        match self {
            &MemoRefPtr::Resident(_) => MemoPeeringStatus::Resident,
            &MemoRefPtr::Remote      => MemoPeeringStatus::Participating
        }
    }
}

impl MemoRef {
    pub fn to_head (&self) -> MemoRefHead {
        MemoRefHead::from_memoref(self.clone())
    }
    pub fn apply_peers ( &self, apply_peerlist: &MemoPeerList ) -> bool {

        let mut peerlist = &mut *self.peerlist.write().unwrap();
        let mut acted = false;
        for apply_peer in apply_peerlist.0.clone() {
            if peerlist.apply_peer(apply_peer) {
                acted = true;
            }
        }
        acted
    }
    pub fn get_peerlist_for_peer (&self, my_ref: &SlabRef, dest_slabref: &SlabRef) -> MemoPeerList {
        let mut list : Vec<MemoPeer> = Vec::new();

        list.push(MemoPeer{
            slabref: my_ref.clone(),
            status: self.ptr.read().unwrap().to_peering_status()
        });

        // Tell the peer about all other presences except for ones belonging to them
        // we don't need to tell them they have it. They know, they were there :)

        for peer in self.peerlist.read().unwrap().iter().filter(|p| p.slabref.0.slab_id != dest_slabref.0.slab_id ) {
            list.push((*peer).clone());
        }

        MemoPeerList::new(list)

    }
    pub fn is_resident(&self) -> bool {
        match *self.ptr.read().unwrap() {
            MemoRefPtr::Resident(_) => true,
            _                       => false
        }
    }
    pub fn get_memo_if_resident(&self) -> Option<Memo> {
        match *self.ptr.read().unwrap() {
            MemoRefPtr::Resident(ref memo) => Some(memo.clone()),
            _ => None
        }
    }
    pub fn is_peered_with_slabref(&self, slabref: &SlabRef) -> bool {
        let status = self.peerlist.read().unwrap().iter().any(|peer| {
            (peer.slabref.0.slab_id == slabref.0.slab_id && peer.status != MemoPeeringStatus::NonParticipating)
        });

        status
    }
    pub fn get_memo (&self, slab: &Slab) -> Result<Memo,RetrieveError> {
//        println!("Slab({}).MemoRef({}).get_memo()", self.owning_slab_id, self.id );
        assert!(self.owning_slab_id == slab.id,"requesting slab does not match owning slab");

        // This seems pretty crude, but using channels for now in the interest of expediency
        let channel;
        {
            if let MemoRefPtr::Resident(ref memo) = *self.ptr.read().unwrap() {
                return Ok(memo.clone());
            }

                println!("MARK X");
            if slab.request_memo(self) > 0 {
                channel = slab.memo_wait_channel(self.id);
            }else{
                println!("MARK Y");
                return Err(RetrieveError::NotFound)
            }
        }

            println!("MARK Z");

        // By sending the memo itself through the channel
        // we guarantee that there's no funny business with request / remotize timing


        use std::time;
        let timeout = time::Duration::from_millis(2000);

        for _ in 0..3 {
            match channel.recv_timeout(timeout) {
                Ok(memo)       =>{
                    println!("Slab({}).MemoRef({}).get_memo() received memo: {}", self.owning_slab_id, self.id, memo.id );
                    return Ok(memo)
                }
                Err(rcv_error) => {

                    use std::sync::mpsc::RecvTimeoutError::*;
                    match rcv_error {
                        Timeout => {}
                        Disconnected => {
                            return Err(RetrieveError::SlabError)
                        }
                    }
                }
            }

            // have another go around
            if slab.request_memo( &self ) == 0 {
                return Err(RetrieveError::NotFound)
            }

        }

        Err(RetrieveError::NotFoundByDeadline)

    }
    pub fn descends (&self, memoref: &MemoRef, slab: &Slab) -> bool {
        assert!(self.owning_slab_id == slab.id);
        match self.get_memo( slab ) {
            Ok(my_memo) => {
                if my_memo.descends(&memoref, slab) {
                    return true }
            }
            Err(_) => {
                panic!("Unable to retrieve my memo")
            }
        };

        false
    }
    pub fn update_peer (&self, slabref: &SlabRef, status: MemoPeeringStatus){

        let mut found : bool = false;
        let ref mut list = self.peerlist.write().unwrap().0;
        for peer in list.iter_mut() {
            if peer.slabref.slab_id == slabref.slab_id {
                found = true;
                peer.status = status.clone();
                // TODO remove the peer entirely for MemoPeeringStatus::NonParticipating
                // TODO prune excess peers - Should keep this list O(10) peers
            }
        }

        if !found {
            list.push(MemoPeer{
                slabref: slabref.clone(),
                status: status.clone()
            })
        }
    }
    pub fn clone_for_slab (&self, from_slabref: &SlabRef, to_slab: &Slab, include_memo: bool ) -> Self{
        assert!(from_slabref.owning_slab_id == to_slab.id,"MemoRef clone_for_slab owning slab should be identical");
        assert!(from_slabref.slab_id != to_slab.id,       "MemoRef clone_for_slab dest slab should not be identical");

        let peerlist = self.peerlist.read().unwrap().clone_for_slab( from_slabref, to_slab );

        let maybe_memo;

        {
            let ptr = *self.ptr.read().unwrap();

            maybe_memo = match include_memo {
                true => match ptr {
                    MemoRefPtr::Resident(ref m) => Some(m.clone_for_slab(from_slabref, to_slab)),
                    MemoRefPtr::Remote      => None
                },
                false => None
            };

            peerlist.apply_peer(MemoPeer{
                slabref: from_slabref.clone(),
                status:  ptr.to_peering_status(),
            });
        }

        let memoref = to_slab.assert_memoref(
            self.id,
            self.subject_id,
            peerlist.clone(),
            maybe_memo
        ).0;


        println!("MemoRef.clone_for_slab({},{}) peerlist: {:?} -> MemoRef({:?})", from_slabref.slab_id, to_slab.id, &peerlist, &memoref );

        memoref
    }
}

impl fmt::Debug for MemoRef{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("MemoRef")
           .field("id", &self.id)
           .field("owning_slab_id", &self.owning_slab_id)
           .field("subject_id", &self.subject_id)
           .field("peerlist", &*self.peerlist.read().unwrap())

           .field("resident", &match *self.ptr.read().unwrap() {
               MemoRefPtr::Remote      => false,
               MemoRefPtr::Resident(_) => true
           })
           .finish()
    }
}

impl PartialEq for MemoRef {
    fn eq(&self, other: &MemoRef) -> bool {
        // TODO: handle the comparision of pre-hashed memos as well as hashed memos
        self.id == other.id
    }
}

impl Drop for MemoRefInner{
    fn drop(&mut self) {
        println!("# MemoRefInner({}).drop", self.id);
    }
}
