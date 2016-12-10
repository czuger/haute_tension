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
    # Minus action :
    if params[ :action_code ] == 'minus'
      @adventure.decrement!( params[ :code ] )
    elsif params[ :action_code ] == 'plus'
      @adventure.increment!( params[ :code ] )
    end
    head :ok
  end

  private

  def set_adventure
    @adventure = Adventure.find( params[ :adventure_id ] )
  end

end
