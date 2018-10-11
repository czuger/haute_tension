class HerosController < ApplicationController

  before_action :set_adventure

  VALUES = [
    { name: :hp, base_col: :hp, max_col: :hp_max }, { name: :or, base_col: :gold },
    { name: :rations, base_col: :rations }, { name: :gourdes, base_col: :waterskins, max_col: :waterskins_max },
    { name: :force, base_col: :strength, max_col: :strength_max }, { name: :charisme, base_col: :charisma_avaliable }
  ]

  ACTIONS = [ { name: :hp, code: :hp }, { name: :or, code: :gold }, { name: :rations, code: :rations },
              { name: :gourdes, code: :waterskins }, { name: :hp_max, code: :hp_max }, { name: :force, code: :strength },
              { name: :force_max, code: :strength_max }, { name: :gourdes_max, code: :waterskins_max } ]

  def show
    @data = []
    VALUES.each do |value|
      h = value.clone
      h[ :base_value ] = @adventure.send( value[ :base_col ] )
      h[ :base_value ] = 'Non utilisé' if h[ :base_col ] == true
      h[ :max_value ] = @adventure.send( value[ :max_col ] ) if value[ :max_col ]
      @data << h
    end
  end

  def update
    respond_to do |format|
      if @adventure.update(hero_params )
        format.html { redirect_to adventure_heros_path( @adventure ), notice: 'Données mises à jour.' }
      else
        format.html { render adventure_heros_path( @adventure ) }
      end
    end
  end

  private

  def set_adventure
    @adventure = Adventure.find( params[ :adventure_id ] )
  end

  def hero_params
    params.require( :adventure ).permit( :hp, :strength, :gold, :waterskins, :waterskins_max, :strength_max, :rations, :hp_max, :charisma_avaliable )
  end

end