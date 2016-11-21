class AdventureTrackersController < ApplicationController

  before_action :set_adventure

  def edit
    @edit_action = params[ :edit_action ]
    @edit_typ = params[ :edit_typ ]
  end

  def update
    adventure_params = params.permit( :id, :hp, :hp_max, :strength, :strength_max, :gold, :rations, :waterskins, :waterskins_max, :charisma_avaliable )

    modifications = adventure_params[ :charisma_avaliable ]

    params_keys = adventure_params.keys - %w( id charisma_avaliable )
    params_keys.each do |field|
      amount = adventure_params[field] && adventure_params[field].to_i

      if amount && amount != 0
        amount = adventure_params[field].to_i
        new_value = ( @adventure.send( field ) + ( params['edit_action'] == 'up' ? amount : -amount ) )
        adventure_params[field] = new_value

        if field =~ /max/ && params['edit_action'] == 'down'
          linked_field_name = field[ 0..-5 ].to_s
          adventure_params[ linked_field_name ] = [ @adventure.send( linked_field_name ), adventure_params[ field ] ].min
        end

        @adventure.game_logs.create!( page_id: @adventure.page_id, log_type: GameLog::ADVENTURE_UPDATE,
           log_data: { edit_action: params['edit_action'], edit_typ: params['edit_typ'], amount: amount } )

        modifications = true
      end
    end

    respond_to do |format|
      if modifications
        if @adventure.update( adventure_params )
          format.html { redirect_to adventure_play_url( @adventure ), notice: 'Aventure was successfully updated.' }
        else
          format.html { render :edit }
        end
      else
        format.html { redirect_to adventure_play_url( adventure_id: @adventure ) }
      end
    end
  end

  def set_adventure
    @adventure = Adventure.find(params[:id])
  end

end
